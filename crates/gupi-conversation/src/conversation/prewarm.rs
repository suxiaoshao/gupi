//! A single hidden temporary conversation; never a second process manager.
use super::*;

#[derive(Default)]
pub(super) struct Prewarm {
    enabled: bool,
    requested: bool,
    pub slot: Option<Standby>,
}

pub(super) struct Standby {
    pub key: String,
    command: PathBuf,
    pub resources: Option<Resources>,
    _timeout: Task<()>,
}
#[derive(Clone)]
pub(super) struct Resources {
    started: SystemTime,
    pub executable: PathBuf,
    target: PathBuf,
    files: Vec<(PathBuf, bool)>,
}
impl Resources {
    fn capture(
        started: SystemTime,
        executable: PathBuf,
        agent: PathBuf,
        cwd: PathBuf,
    ) -> Result<Self, String> {
        let target = std::fs::canonicalize(&executable).map_err(|e| e.to_string())?;
        let mut files = Vec::new();
        for path in ["settings.json", "models.json", "auth.json", "mcp.json"]
            .map(|name| agent.join(name))
            .into_iter()
            .chain([
                cwd.join(".pi/settings.json"),
                cwd.join(".pi/mcp.json"),
                executable.clone(),
                target.clone(),
            ])
        {
            let present = match std::fs::symlink_metadata(&path) {
                Ok(_) => true,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
                Err(error) => return Err(error.to_string()),
            };
            files.push((path, present));
        }
        let resources = Self {
            started,
            executable,
            target,
            files,
        };
        resources.check(None)?;
        Ok(resources)
    }
    fn check(&self, template: Option<&std::path::Path>) -> Result<(), String> {
        if std::fs::canonicalize(&self.executable).map_err(|e| e.to_string())? != self.target {
            return Err("Pi executable target changed".into());
        }
        for (path, present) in &self.files {
            match std::fs::symlink_metadata(path) {
                Ok(metadata) if *present => {
                    if metadata.modified().map_err(|e| e.to_string())? > self.started
                        || std::fs::metadata(path)
                            .and_then(|m| m.modified())
                            .map_err(|e| e.to_string())?
                            > self.started
                    {
                        return Err("Pi resource changed after prewarming".into());
                    }
                }
                Err(error) if !present && error.kind() == std::io::ErrorKind::NotFound => {}
                _ => return Err("Pi resource existence changed or cannot be confirmed".into()),
            }
        }
        if let Some(template) = template {
            for metadata in [
                std::fs::symlink_metadata(template),
                std::fs::metadata(template),
            ] {
                let metadata = metadata.map_err(|e| e.to_string())?;
                if metadata.modified().map_err(|e| e.to_string())? > self.started {
                    return Err("Prompt template changed after prewarming".into());
                }
            }
        }
        Ok(())
    }
}
impl Session {
    pub fn is_internal(&self) -> bool {
        self.internal
    }
    fn standby_ready(&self) -> bool {
        self.state.is_some()
            && matches!(self.transcript, Transcript::Ready(_))
            && !self.core_read.running()
            && !self.settings_busy()
            && !self.model_change.unconfirmed()
            && self.pending_ui.is_empty()
            && self.empty_conversation()
            && self.draft.text().is_empty()
            && self.error.is_none()
            && self.core_read.error().is_none()
    }
}
impl ConversationState {
    pub fn set_prewarm(&mut self, enabled: bool, cx: &mut Context<Self>) {
        let changed_command = self
            .prewarm
            .slot
            .as_ref()
            .is_some_and(|slot| slot.command != self.command);
        let changed = self.prewarm.enabled != enabled;
        self.prewarm.enabled = enabled && self.temporary && !self.draining;
        if changed || changed_command {
            self.prewarm.requested = self.prewarm.enabled;
        }
        if !self.prewarm.enabled || changed_command {
            self.discard_standby(cx);
        }
        self.start_prewarm(cx);
    }
    pub(super) fn discard_standby(&mut self, cx: &mut Context<Self>) {
        if let Some(slot) = self.prewarm.slot.take() {
            self.remove_temporary(&slot.key, true, cx);
        }
    }
    pub(super) fn start_prewarm(&mut self, cx: &mut Context<Self>) {
        if self.draining
            || !self.prewarm.enabled
            || !self.prewarm.requested
            || self.prewarm.slot.is_some()
            || self.sessions.values().any(|session| session.internal)
        {
            return;
        }
        self.prewarm.requested = false;
        let Ok(key) = self.create_draft(None) else {
            tracing::warn!("Could not create standby conversation");
            return;
        };
        let timeout_key = key.clone();
        let timeout = cx.spawn(async move |owner, cx| {
            cx.background_executor()
                .timer(Duration::from_secs(30))
                .await;
            let _ = owner.update(cx, |this, cx| {
                if this
                    .prewarm
                    .slot
                    .as_ref()
                    .is_some_and(|slot| slot.key == timeout_key)
                    && this
                        .sessions
                        .get(&timeout_key)
                        .is_some_and(|s| !s.standby_ready())
                {
                    tracing::warn!("Standby conversation did not become ready");
                    this.discard_standby(cx);
                }
            });
        });
        self.prewarm.slot = Some(Standby {
            key: key.clone(),
            command: self.command.clone(),
            resources: None,
            _timeout: timeout,
        });
        let started = SystemTime::now();
        let session = self.sessions.get_mut(&key).unwrap();
        session.internal = true;
        session.core_read = CoreRead::PreparingDirectory;
        let directory = session.info.cwd.clone();
        let agent = self.discovery.agent.clone();
        let environment = gupi_pi_runtime::environment(cx);
        let options = LaunchOptions::new(self.command.clone(), directory.clone());
        let executable = environment.resolve(options, cx);
        session.directory = Some(cx.spawn(async move |owner, cx| {
            let result = async {
                let executable = executable.await.map_err(|e| e.to_string())?;
                smol::unblock(move || {
                    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
                    Resources::capture(started, executable, agent, directory)
                })
                .await
            }
            .await;
            let _ = owner.update(cx, |this, cx| {
                let Some(slot) = this.prewarm.slot.as_mut().filter(|slot| slot.key == key) else {
                    return;
                };
                this.sessions.get_mut(&key).unwrap().directory = None;
                this.sessions.get_mut(&key).unwrap().core_read.finish(None);
                match result {
                    Ok(resources) => {
                        slot.resources = Some(resources);
                        this.launch(&key, cx);
                    }
                    Err(error) => {
                        tracing::warn!(%error, "Could not prepare standby conversation");
                        this.discard_standby(cx);
                    }
                }
            });
        }));
    }
    pub(super) fn maintain_prewarm(&mut self, key: &str, cx: &mut Context<Self>) {
        let Some(session) = self.sessions.get(key) else {
            return;
        };
        if session.internal {
            if session.error.is_some()
                || session.core_read.error().is_some()
                || !session.pending_ui.is_empty()
                || !session.empty_conversation()
                || !session.draft.text().is_empty()
            {
                tracing::warn!("Discarding unusable standby conversation");
                self.discard_standby(cx);
            }
        } else if session.replenish
            && !session.preparing
            && session.state.is_some()
            && matches!(session.transcript, Transcript::Ready(_))
            && !session.core_read.running()
            && session.error.is_none()
            && session.core_read.error().is_none()
        {
            self.sessions.get_mut(key).unwrap().replenish = false;
            self.prewarm.requested = self.prewarm.enabled;
            self.start_prewarm(cx);
        }
    }
    /// Metadata I/O precedes one owner update that adopts or creates and selects.
    pub fn new_temporary(
        &mut self,
        template: Option<PathBuf>,
        preparing: bool,
        cx: &mut Context<Self>,
    ) -> Task<Result<String, String>> {
        if !self.temporary || self.draining {
            return Task::ready(Err("Temporary conversations are unavailable".into()));
        }
        let candidate = self.prewarm.slot.as_ref().and_then(|slot| {
            let session = self.sessions.get(&slot.key)?;
            (self.prewarm.enabled
                && session.standby_ready()
                && self
                    .client(&slot.key, cx)
                    .is_some_and(|client| matches!(client.state(), ConnectionState::Ready(_))))
            .then(|| {
                slot.resources
                    .clone()
                    .map(|resources| (slot.key.clone(), session.binding, resources))
            })
            .flatten()
        });
        let Some((key, binding, resources)) = candidate else {
            return Task::ready(self.cold_temporary(preparing, cx));
        };
        cx.spawn(async move |owner, cx| {
            let fresh = smol::unblock(move || resources.check(template.as_deref())).await;
            owner
                .update(cx, |this, cx| {
                    if this.draining {
                        return Err("Temporary conversation was cancelled".into());
                    }
                    let same =
                        this.prewarm.enabled
                            && this.prewarm.slot.as_ref().is_some_and(|slot| {
                                slot.key == key && slot.command == this.command
                            })
                            && this
                                .sessions
                                .get(&key)
                                .is_some_and(|s| s.binding == binding && s.standby_ready())
                            && this.client(&key, cx).is_some_and(|client| {
                                matches!(client.state(), ConnectionState::Ready(_))
                            });
                    if same && fresh.is_ok() {
                        tracing::info!("Adopted prewarmed temporary conversation");
                        this.prewarm.slot = None;
                        this.activate_temporary(&key, preparing, cx);
                        Ok(key)
                    } else {
                        if this
                            .prewarm
                            .slot
                            .as_ref()
                            .is_some_and(|slot| slot.key == key)
                        {
                            if let Err(error) = fresh {
                                tracing::info!(%error, "Standby resources are stale");
                            }
                            this.discard_standby(cx);
                        }
                        this.cold_temporary(preparing, cx)
                    }
                })
                .map_err(|e| e.to_string())?
        })
    }
    fn cold_temporary(
        &mut self,
        preparing: bool,
        cx: &mut Context<Self>,
    ) -> Result<String, String> {
        let key = self.create_draft(None)?;
        self.activate_temporary(&key, preparing, cx);
        self.connect(&key, cx);
        Ok(key)
    }
    fn activate_temporary(&mut self, key: &str, preparing: bool, cx: &mut Context<Self>) {
        let session = self.sessions.get_mut(key).unwrap();
        session.internal = false;
        session.replenish = true;
        session.preparing = preparing;
        self.select_existing(key, cx);
        notify_session(key, cx);
        self.maintain_prewarm(key, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::{ConversationState, Resources, Transcript};
    use gpui_kit::{AppContext as _, TestAppContext};
    use std::prelude::v1::test;
    use std::time::{Duration, SystemTime};

    #[test]
    fn changed_template_config_and_executable_cannot_use_standby_resources() {
        let fixture = tempfile::tempdir().unwrap();
        let root = fixture.path();
        let executable = root.join("pi");
        let template = root.join("translate.md");
        std::fs::write(&executable, "fixture executable").unwrap();
        std::fs::write(&template, "old template").unwrap();
        let started = SystemTime::now();
        let resources =
            Resources::capture(started, executable.clone(), root.into(), root.into()).unwrap();
        assert!(resources.check(Some(&template)).is_ok());
        std::fs::File::options()
            .write(true)
            .open(&template)
            .unwrap()
            .set_modified(started + Duration::from_secs(2))
            .unwrap();
        assert!(
            resources.check(Some(&template)).is_err(),
            "a newly edited template requires a cold process"
        );
        let settings = root.join("settings.json");
        std::fs::write(&settings, "{}").unwrap();
        assert!(
            resources.check(None).is_err(),
            "previously missing configuration cannot silently appear"
        );
        std::fs::remove_file(settings).unwrap();
        let project_mcp = root.join(".pi/mcp.json");
        std::fs::create_dir(root.join(".pi")).unwrap();
        std::fs::write(&project_mcp, "{}").unwrap();
        assert!(
            resources.check(None).is_err(),
            "new project MCP configuration requires a cold process"
        );
        std::fs::remove_file(project_mcp).unwrap();
        std::fs::File::options()
            .write(true)
            .open(&executable)
            .unwrap()
            .set_modified(started + Duration::from_secs(2))
            .unwrap();
        assert!(
            resources.check(None).is_err(),
            "same-path Pi replacements invalidate standby"
        );
    }

    #[gpui_kit::test]
    fn nonselected_creation_and_adoption_preserve_loaded_conversation(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_kit::init(cx);
            crate::host::install_headless(cx);
            gupi_pi_runtime::init(cx);
        });
        let owner = cx.new(|cx| ConversationState::temporary("unused-pi".into(), cx));
        owner.update(cx, |state, cx| {
            let key = state.create_draft(None).unwrap();
            assert!(state.selected.is_none(), "background creation never changes foreground selection");
            let session = state.sessions.get_mut(&key).unwrap();
            session.internal = true;
            session.state = Some(serde_json::from_value(serde_json::json!({"sessionId":"ready", "isStreaming":false, "isCompacting":false})).unwrap());
            session.transcript.replace(serde_json::from_value(serde_json::json!({"entries":[], "leafId":null})).unwrap());
            assert!(state.infos().is_empty());
            state.select_existing(&key, cx);
            assert!(state.selected.is_none(), "internal keys cannot be navigated to");
            state.activate_temporary(&key, true, cx);
            assert_eq!(state.selected.as_deref(), Some(key.as_str()));
            let session = &state.sessions[&key];
            assert!(!session.internal);
            assert!(session.preparing);
            assert_eq!(session.state.as_ref().unwrap().session_id, "ready");
            assert!(matches!(session.transcript, Transcript::Ready(_)));
            assert_eq!(state.infos().len(), 1);
        });
    }
}
