//! Application ownership of independent Pi connections. No conversation UI policy.
use gpui_kit::*;
use gpui_tokio::Tokio;
use pi_rpc::{
    Client, CloseReport, ConnectionState, Error, EventStream, LaunchOptions, protocol::Event,
};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct InstanceId(u64);
struct Connection {
    client: Client,
    _consumer: Task<()>,
    _extension: tempfile::TempDir,
}

#[cfg(all(test, unix))]
mod tests {
    use super::{Instance, global, init};
    use gpui_kit::TestAppContext;
    use pi_rpc::LaunchOptions;
    use std::os::unix::fs::PermissionsExt as _;
    use std::prelude::v1::test;

    #[gpui_kit::test]
    async fn closing_during_protocol_startup_confirms_process_exit(cx: &mut TestAppContext) {
        cx.executor().allow_parking();
        cx.update(|cx| {
            gpui_tokio::init(cx);
            init(cx);
        });
        let fixture = tempfile::tempdir().unwrap();
        let executable = fixture.path().join("pi-fixture");
        std::fs::write(&executable, r#"#!/bin/sh
printf started > "$GUPI_FIXTURE_STARTED"
printf '%s\n' '{"type":"fixture_started"}'
while IFS= read -r line; do
  case "$line" in *"close"*) printf '%s\n' '{"type":"response","id":"close","command":"abort","success":true,"data":{}}';; esac
done
"#).unwrap();
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
        let owner = cx.read(global);
        for handoff in [false, true] {
            let marker = fixture
                .path()
                .join(if handoff { "handoff" } else { "cancel" });
            let mut options = LaunchOptions::new(&executable, fixture.path());
            options.env.push((
                "GUPI_FIXTURE_STARTED".into(),
                marker.clone().into_os_string(),
            ));
            let id = owner.update(cx, |state, cx| state.start(options, cx).unwrap());
            if handoff {
                cx.condition(&owner, |state, _| {
                    matches!(state.instances.get(&id), Some(Instance::Connected(_)))
                })
                .await;
            }
            let closing = owner.update(cx, |state, cx| state.close(id, cx));
            let repeated = owner.update(cx, |state, cx| state.close(id, cx));
            let all = handoff.then(|| owner.update(cx, |state, cx| state.close_all(cx)));
            assert!(owner.read_with(cx, |state, _| {
                matches!(state.instances.get(&id), Some(Instance::Closing { .. }))
                    && matches!(state.client(id), Err(pi_rpc::Error::Closed))
            }));
            let report = closing.await;
            assert!(
                report.as_ref().is_none_or(|report| report.status.is_some()),
                "cancel must either prevent spawn or confirm process exit: {report:?}"
            );
            if handoff {
                assert!(
                    report.is_some(),
                    "a handed-off child must have an exit report"
                );
            }
            if report.is_none() {
                assert!(
                    !marker.exists(),
                    "no exit report is valid only when the child never started"
                );
            }
            let repeated = repeated.await;
            assert_eq!(
                repeated.as_ref().map(|report| report.status),
                report.as_ref().map(|report| report.status),
                "repeated close waits for the same exit result"
            );
            if let Some(all) = all {
                let reports = all.await;
                assert_eq!(
                    reports.len(),
                    1,
                    "close_all includes the already closing child"
                );
                assert!(reports[0].status.is_some());
            }
            assert!(owner.read_with(cx, |state, _| !state.instances.contains_key(&id)));
        }
    }
}
enum Instance {
    Launching {
        _task: Task<()>,
        cancel: tokio::sync::watch::Sender<bool>,
        done: tokio::sync::oneshot::Receiver<Option<CloseReport>>,
    },
    Connected(Connection),
    Failed(Error),
    Closing {
        _task: Task<()>,
        result: tokio::sync::watch::Receiver<CloseState>,
    },
}

#[derive(Clone)]
enum CloseState {
    Pending,
    Complete(Option<CloseReport>),
}

/// Emitted without storing a second transcript or an unbounded event history.
pub struct PiEvent {
    instance: InstanceId,
    event: Event,
}
impl PiEvent {
    pub fn instance(&self) -> InstanceId {
        self.instance
    }
    pub fn event(&self) -> &Event {
        &self.event
    }
}
pub struct ConnectionChanged(InstanceId);
impl ConnectionChanged {
    pub fn instance(&self) -> InstanceId {
        self.0
    }
}
impl EventEmitter<ConnectionChanged> for PiState {}
pub struct PiState {
    instances: BTreeMap<InstanceId, Instance>,
    next_id: u64,
    draining: bool,
}
impl EventEmitter<PiEvent> for PiState {}
struct GlobalPi(Entity<PiState>);
impl Global for GlobalPi {}

pub fn init(cx: &mut App) {
    let state = cx.new(|_| PiState {
        instances: BTreeMap::new(),
        next_id: 0,
        draining: false,
    });
    cx.set_global(GlobalPi(state));
}
pub fn global(cx: &App) -> Entity<PiState> {
    cx.global::<GlobalPi>().0.clone()
}

impl PiState {
    pub fn start(
        &mut self,
        mut options: LaunchOptions,
        cx: &mut Context<Self>,
    ) -> Result<InstanceId, Error> {
        if self.draining {
            return Err(Error::Closed);
        }
        self.next_id += 1;
        let id = InstanceId(self.next_id);
        let environment = super::environment::current(cx);
        let (cancel, mut cancellation) = tokio::sync::watch::channel(false);
        let launch = Tokio::spawn(cx, async move {
            tokio::select! {
                biased;
                _ = cancellation.changed() => Err(Error::Closed),
                result = async {
            let snapshot = environment.load(false).await;
            if !options.clear_env {
                let mut variables = snapshot.variables();
                variables.append(&mut options.env);
                options.env = variables;
            }
            let extension = super::extension::install(&mut options)?;
            Client::spawn(options)
                .await
                .map(|(client, events)| (client, events, extension))
                .map_err(|error| match error {
                    Error::Io(error) => Error::Io(snapshot.explain(error)),
                    error => error,
                })
                } => result,
            }
        });
        let cancelling = cancel.clone();
        let (complete, done) = tokio::sync::oneshot::channel();
        let task = cx.spawn(async move |owner, cx| {
            let mut result = Some(
                launch
                    .await
                    .unwrap_or_else(|error| Err(Error::Io(error.to_string()))),
            );
            let _ = owner.update(cx, |owner, cx| {
                if !owner.instances.contains_key(&id) || owner.draining || *cancelling.borrow() {
                    return;
                }
                let instance = match result.take().unwrap() {
                    Ok((client, events, extension)) => Instance::Connected(Connection {
                        _extension: extension,
                        _consumer: Self::consume(id, client.clone(), events, cx),
                        client,
                    }),
                    Err(error) => {
                        tracing::warn!(instance = ?id, stage = "process_launch", category = error.category(), failed = true, "Pi failure");
                        Instance::Failed(error)
                    },
                };
                owner.instances.insert(id, instance);
                cx.emit(ConnectionChanged(id));
                cx.notify();
            });
            let report = if let Some(Ok((client, _events, extension))) = result {
                let report = client.close().await;
                drop(extension);
                Some(report)
            } else {
                None
            };
            let _ = complete.send(report);
        });
        self.instances.insert(
            id,
            Instance::Launching {
                _task: task,
                cancel,
                done,
            },
        );
        cx.notify();
        Ok(id)
    }
    fn consume(
        id: InstanceId,
        client: Client,
        mut events: EventStream,
        cx: &mut Context<Self>,
    ) -> Task<()> {
        let mut status = client.subscribe();
        cx.spawn(async move |owner, cx| {
            let mut events_open = true;
            loop {
                // Tokio channels can be awaited on GPUI's executor; process I/O stays
                // on the Tokio owner. Only one delivered event is held on this bridge.
                tokio::select! {
                    event = events.recv(), if events_open => {
                        let Some(event) = event else {
                            // The event sender can close before the final status
                            // notification. Keep observing until owners learn the
                            // connection has exited and can reconnect it.
                            events_open = false;
                            continue;
                        };
                        if owner.update(cx, |_, cx| {
                            #[cfg(feature = "performance")]
                            let _span = tracing::debug_span!(target: "gupi::performance", "rpc.deliver_event", kind = event.raw()["type"].as_str().unwrap_or("unknown")).entered();
                            cx.emit(PiEvent { instance: id, event });
                        }).is_err() { break; }
                    }
                    changed = status.changed() => {
                        if changed.is_err() { break; }
                        let exited = matches!(*status.borrow_and_update(), ConnectionState::Closed(_));
                        if owner.update(cx, |_, cx| { cx.emit(ConnectionChanged(id)); cx.notify(); }).is_err() { break; }
                        if exited {
                            // Drain any events queued before the exit report.
                            while let Some(event) = events.recv().await {
                                if owner.update(cx, |_, cx| cx.emit(PiEvent { instance: id, event })).is_err() { break; }
                            }
                            break;
                        }
                    }
                }
            }
        })
    }
    pub fn client(&self, id: InstanceId) -> Result<Option<Client>, Error> {
        match self.instances.get(&id) {
            Some(Instance::Connected(connection)) => Ok(Some(connection.client.clone())),
            Some(Instance::Launching { .. }) => Ok(None),
            Some(Instance::Failed(error)) => Err(error.clone()),
            Some(Instance::Closing { .. }) => Err(Error::Closed),
            None => Err(Error::Closed),
        }
    }
    pub fn close(&mut self, id: InstanceId, cx: &mut Context<Self>) -> Task<Option<CloseReport>> {
        if let Some(Instance::Closing { result, .. }) = self.instances.get(&id) {
            return Self::wait_for_close(result.clone(), cx);
        }
        let instance = self.instances.remove(&id);
        let Some(instance) = instance else {
            return Task::ready(None);
        };
        if let Instance::Launching { cancel, .. } = &instance {
            cancel.send_replace(true);
        }
        let closing = match &instance {
            Instance::Connected(connection) => Some(connection.client.close()),
            _ => None,
        };
        let (complete, result) = tokio::sync::watch::channel(CloseState::Pending);
        let task = cx.spawn(async move |owner, cx| {
            let report = match (instance, closing) {
                (Instance::Connected(connection), Some(closing)) => {
                    // Keep the event consumer alive until the process owner is done.
                    let report = closing.await;
                    drop(connection);
                    Some(report)
                }
                (Instance::Launching { _task, done, .. }, _) => {
                    let report = done.await.unwrap_or_else(|_| {
                        Some(CloseReport {
                            status: None,
                            reason: Some(Error::Closed),
                            stderr: String::new(),
                        })
                    });
                    drop(_task);
                    report
                }
                _ => None,
            };
            let _ = owner.update(cx, |owner, cx| {
                owner.instances.remove(&id);
                cx.notify();
            });
            complete.send_replace(CloseState::Complete(report));
        });
        self.instances.insert(
            id,
            Instance::Closing {
                _task: task,
                result: result.clone(),
            },
        );
        cx.emit(ConnectionChanged(id));
        cx.notify();
        Self::wait_for_close(result, cx)
    }
    fn wait_for_close(
        mut result: tokio::sync::watch::Receiver<CloseState>,
        cx: &mut Context<Self>,
    ) -> Task<Option<CloseReport>> {
        cx.spawn(async move |_, _| {
            loop {
                if let CloseState::Complete(report) = &*result.borrow_and_update() {
                    return report.clone();
                }
                if result.changed().await.is_err() {
                    return Some(CloseReport {
                        status: None,
                        reason: Some(Error::Closed),
                        stderr: String::new(),
                    });
                }
            }
        })
    }
    pub fn close_all(&mut self, cx: &mut Context<Self>) -> Task<Vec<CloseReport>> {
        self.draining = true;
        let ids: Vec<_> = self.instances.keys().copied().collect();
        let connections: Vec<_> = ids.into_iter().map(|id| self.close(id, cx)).collect();
        cx.notify();
        cx.spawn(async move |_, _| {
            let mut reports = Vec::new();
            for closing in connections {
                let Some(report) = closing.await else {
                    continue;
                };
                if let Some(error) = &report.reason {
                    tracing::warn!(%error, "Pi connection closed with an error");
                }
                reports.push(report);
            }
            reports
        })
    }
}
