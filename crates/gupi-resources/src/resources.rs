use crate::pi_resources as io;
use crate::pi_resources::Baseline;
use crate::pi_resources::Catalog;
use crate::pi_resources::Error;
use crate::pi_resources::Kind;
use crate::pi_resources::Resource;
use gpui_kit::*;
use gpui_operation::Cancel;
use gpui_operation::Complete;
use gpui_operation::Load;
use gpui_operation::Refresh;
use gpui_operation::Retry;
use gpui_operation::Transition;
use gpui_operation::refresh;
use gpui_tokio::Tokio;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub enum Change {
    Package {
        command: PathBuf,
        action: &'static str,
        source: String,
    },
    Toggle(Resource, bool),
    RegisterSkill(PathBuf),
    Save {
        path: PathBuf,
        text: String,
        create: bool,
        baseline: Baseline,
    },
    Delete(Resource),
}

/// Whose files a controller manages.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    /// The personal agent directory and its packages.
    Global,
    /// A project's `.pi` resources and package declarations, identified by its cwd.
    Project(PathBuf),
}
/// A mutation owns its request and task together, so activity cannot outlive the task.
#[derive(Default)]
pub enum Mutation {
    #[default]
    Idle,
    Running {
        change: Change,
        task: Task<()>,
    },
}
impl Mutation {
    pub fn is_running(&self) -> bool {
        matches!(self, Self::Running { .. })
    }

    pub fn package_running(&self, source: &str, action: &str) -> bool {
        matches!(self, Self::Running {
            change: Change::Package { source: active_source, action: active_action, .. }, ..
        } if active_source == source && *active_action == action)
    }
}

pub struct ResourceController {
    target: Target,
    catalog: refresh::Operation<Catalog, Error, Task<()>>,
    mutation: Mutation,
    shutting_down: bool,
    refresh_pending: bool,
}
pub enum ResourceEvent {
    CatalogChanged,
    Saved(PathBuf, String),
    Finished {
        target: String,
        result: Result<(), Error>,
        change: Change,
    },
}
impl EventEmitter<ResourceEvent> for ResourceController {}
impl Default for ResourceController {
    fn default() -> Self {
        Self::new()
    }
}
impl ResourceController {
    pub fn catalog(&self) -> &refresh::Operation<Catalog, Error, Task<()>> {
        &self.catalog
    }
    pub fn mutation(&self) -> &Mutation {
        &self.mutation
    }
    pub fn new() -> Self {
        Self::for_target(Target::Global)
    }
    pub fn for_target(target: Target) -> Self {
        Self {
            target,
            catalog: refresh::Operation::new(),
            mutation: Mutation::Idle,
            shutting_down: false,
            refresh_pending: false,
        }
    }
    pub fn target(&self) -> &Target {
        &self.target
    }
    pub fn busy(&self) -> bool {
        self.catalog.is_running() || self.mutation.is_running()
    }
    /// Stop reads and new requests while retaining mutations until they settle.
    pub fn shutdown(&mut self) {
        self.shutting_down = true;
        self.refresh_pending = false;
        self.catalog.transition(Cancel);
    }
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        if self.busy() && !self.shutting_down {
            self.refresh_pending = true;
            return;
        }
        self.refresh_after(None, cx);
    }
    fn refresh_after(&mut self, change: Option<Change>, cx: &mut Context<Self>) {
        if self.shutting_down || self.busy() {
            return;
        }
        let previous = if self.refresh_pending {
            None
        } else {
            change.as_ref().and_then(|_| self.catalog.data().cloned())
        };
        self.refresh_pending = false;
        let environment = gupi_pi_runtime::environment(cx);
        let target = self.target.clone();
        let worker = Tokio::spawn(cx, async move {
            if let (Some(change), Some(mut catalog)) = (change, previous) {
                match change {
                    Change::Toggle(resource, enabled) if target == Target::Global => {
                        for item in &mut catalog.resources {
                            if item.path == resource.path {
                                item.enabled = enabled;
                            }
                        }
                        return Ok(catalog);
                    }
                    Change::Delete(resource) => {
                        catalog.resources.retain(|item| item.path != resource.path);
                        return Ok(catalog);
                    }
                    Change::Save { path, .. } => {
                        if let Some(resource) = catalog
                            .resources
                            .iter()
                            .find(|item| item.path == path)
                            .cloned()
                        {
                            io::reload_resource(&mut catalog, resource);
                            return Ok(catalog);
                        }
                    }
                    _ => {}
                }
            }
            if let Target::Project(cwd) = target {
                return smol::unblock(move || io::scan_project(&cwd)).await;
            }
            let snapshot = environment.load(false).await;
            let root = io::agent_dir().map_err(|error| Error(snapshot.explain(error)))?;
            let variables = snapshot.variables();
            let agents = dirs_next::home_dir().map(|p| p.join(".agents"));
            smol::unblock(move || io::scan(root, agents, &variables))
                .await
                .map_err(|error| Error(snapshot.explain(error)))
        });
        let task = cx.spawn(async move |owner, cx| {
            let result = worker.await.unwrap_or_else(|e| Err(Error(e.to_string())));
            let _ = owner.update(cx, |owner, cx| {
                let changed = result
                    .as_ref()
                    .ok()
                    .is_some_and(|next| owner.catalog.data() != Some(next));
                owner.catalog.transition(Complete(result));
                if changed {
                    cx.emit(ResourceEvent::CatalogChanged);
                }
                if owner.refresh_pending {
                    owner.refresh(cx);
                }
                cx.notify();
            });
        });
        match &self.catalog {
            refresh::Operation::Idle(_) => self.catalog.transition(Load(task)),
            refresh::Operation::Unavailable(_) => self.catalog.transition(Retry(task)),
            _ => self.catalog.transition(Refresh(task)),
        }
        cx.notify();
    }
    pub fn change(&mut self, change: Change, cx: &mut Context<Self>) {
        if self.shutting_down || self.busy() {
            return;
        }
        let Some(root) = self.catalog.data().map(|c| c.root.clone()) else {
            return;
        };
        // Project scope accepts its own file edits, toggles and package commands.
        let project = match &self.target {
            Target::Global => None,
            Target::Project(cwd) => Some(cwd.clone()),
        };
        if project.is_some()
            && !matches!(
                change,
                Change::Save { .. }
                    | Change::Delete(_)
                    | Change::Package { .. }
                    | Change::Toggle(..)
            )
        {
            return;
        }
        let target = match &change {
            Change::Package { source, .. } => source.clone(),
            Change::Toggle(resource, _) | Change::Delete(resource) => resource.name.clone(),
            Change::RegisterSkill(path) | Change::Save { path, .. } => path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
        };
        let saved = match &change {
            Change::Save { path, text, .. } => Some((path.clone(), text.clone())),
            _ => None,
        };
        let updated = change.clone();
        let active = change.clone();
        let environment = gupi_pi_runtime::environment(cx);
        let worker = Tokio::spawn(cx, async move {
            match change {
                Change::Package {
                    command,
                    action,
                    source,
                } => {
                    let snapshot = environment.load(false).await;
                    let result = if let Some(cwd) = project {
                        let agent = io::agent_dir()?;
                        io::project_package_action(
                            command,
                            agent,
                            cwd,
                            action,
                            source,
                            snapshot.variables(),
                        )
                        .await
                    } else {
                        io::package_action(command, root, action, source, snapshot.variables())
                            .await
                    };
                    result.map_err(|error| Error(snapshot.explain(error)))
                }
                change => tokio::task::spawn_blocking(move || {
                    // Re-check ownership at write time, not only when listed.
                    let outside = |path: &std::path::Path| {
                        project
                            .as_ref()
                            .is_some_and(|cwd| !io::project_owns(cwd, path))
                    };
                    match change {
                        Change::Toggle(resource, active) => match &project {
                            Some(cwd) => io::set_project_enabled(cwd, &resource, active),
                            None => io::set_enabled(&root, &resource, active),
                        },
                        Change::RegisterSkill(path) => io::register_skill(&root, &path),
                        Change::Save {
                            path,
                            text,
                            create,
                            baseline,
                        } => match &project {
                            Some(cwd) => {
                                io::save_project_text(cwd, &path, &text, create, &baseline)
                            }
                            None => io::save_text(&path, &text, create, &baseline),
                        },
                        Change::Delete(resource) => {
                            if !resource.editable
                                || resource.package.is_some()
                                || !matches!(resource.kind, Kind::Skill | Kind::Prompt)
                                || outside(&resource.path)
                            {
                                return Err(Error("Resource is read-only".into()));
                            }
                            // Only the listed file: a skill's folder keeps its other files.
                            trash::delete(&resource.path).map_err(|e| Error(e.to_string()))
                        }
                        Change::Package { .. } => unreachable!(),
                    }
                })
                .await
                .map_err(|e| Error(e.to_string()))?,
            }
        });
        let task = cx.spawn(async move |owner, cx| {
            let result = worker.await.unwrap_or_else(|e| Err(Error(e.to_string())));
            let _ = owner.update(cx, |owner, cx| {
                let success = result.is_ok();
                owner.mutation = Mutation::Idle;
                cx.emit(ResourceEvent::Finished {
                    target,
                    result,
                    change: updated.clone(),
                });
                if success && let Some((path, text)) = saved {
                    cx.emit(ResourceEvent::Saved(path, text));
                }
                // A CLI failure may still have changed files. Always reread the actual state.
                owner.refresh_after(success.then_some(updated), cx);
                cx.notify();
            });
        });
        self.mutation = Mutation::Running {
            change: active,
            task,
        };
        cx.notify();
    }
}

#[cfg(any(test, feature = "test-support"))]
impl ResourceController {
    pub fn set_catalog_for_test(&mut self, catalog: Catalog) {
        self.catalog.transition(Load(Task::ready(())));
        self.catalog.transition(Complete(Ok(catalog)));
    }
}

#[cfg(test)]
mod tests {
    use super::{Change, Error, Mutation, ResourceController, Target};
    use gpui_kit::{AppContext, Task, TestAppContext};
    use gpui_operation::{Complete, Load, Transition};

    fn package(source: &str, action: &'static str) -> Change {
        Change::Package {
            command: "pi".into(),
            action,
            source: source.into(),
        }
    }

    #[gpui_kit::test]
    fn unavailable_catalog_never_starts_a_package_mutation(cx: &mut TestAppContext) {
        let owner = cx.new(|_| ResourceController::new());
        owner.update(cx, |owner, cx| {
            owner.catalog.transition(Load(Task::ready(())));
            owner
                .catalog
                .transition(Complete(Err(Error("invalid settings".into()))));
            owner.change(package("npm:example", "install"), cx);
            assert!(!owner.mutation().is_running());
            assert!(!owner.mutation().package_running("npm:example", "install"));
        });
    }

    #[gpui_kit::test]
    fn project_scope_never_registers_global_resources(cx: &mut TestAppContext) {
        let owner = cx.new(|_| ResourceController::for_target(Target::Project("/p".into())));
        owner.update(cx, |owner, cx| {
            owner.catalog.transition(Load(Task::ready(())));
            owner
                .catalog
                .transition(Complete(Ok(crate::pi_resources::Catalog::default())));
            owner.change(Change::RegisterSkill("/p/.pi/skills/a".into()), cx);
            assert!(!owner.mutation().is_running());
        });
    }

    #[test]
    fn package_loading_matches_the_request_and_survives_shutdown() {
        let mut owner = ResourceController::new();
        for action in ["install", "update", "remove"] {
            owner.mutation = Mutation::Running {
                change: package("npm:example", action),
                task: Task::ready(()),
            };
            assert!(owner.mutation().package_running("npm:example", action));
            assert!(!owner.mutation().package_running("npm:other", action));
            for other in ["install", "update", "remove"] {
                assert_eq!(
                    owner.mutation().package_running("npm:example", other),
                    action == other
                );
            }
            owner.shutdown();
            assert!(owner.mutation().is_running());
            assert!(owner.mutation().package_running("npm:example", action));
        }
    }
}

#[cfg(all(test, unix))]
mod shutdown_tests {
    use super::{Change, ResourceController, ResourceEvent, Target};
    use gpui_kit::{AppContext, TestAppContext};
    use gpui_tokio::Tokio;
    use std::{cell::RefCell, os::unix::fs::PermissionsExt, rc::Rc};

    #[gpui_kit::test]
    async fn shutdown_retains_real_package_commands_until_success_or_failure(
        cx: &mut TestAppContext,
    ) {
        cx.executor().allow_parking();
        cx.update(gpui_tokio::init);
        for fail in [false, true] {
            let root = tempfile::tempdir().unwrap();
            let command = root.path().join("fake-pi");
            std::fs::write(&command, "#!/bin/sh\nroot=$PI_CODING_AGENT_DIR\n: > \"$root/started\"\nwhile [ ! -f \"$root/release\" ]; do /bin/sleep 0.01; done\n: > \"$root/finished\"\nif [ -f \"$root/fail\" ]; then exit 9; fi\n").unwrap();
            std::fs::set_permissions(&command, std::fs::Permissions::from_mode(0o700)).unwrap();
            if fail {
                std::fs::write(root.path().join("fail"), "").unwrap();
            }
            let owner = cx.new(|_| ResourceController::new());
            let results = Rc::new(RefCell::new(vec![]));
            let reported = results.clone();
            let _subscription = cx.update(|cx| {
                cx.subscribe(&owner, move |_, event, _| {
                    if let ResourceEvent::Finished { result, .. } = event {
                        reported.borrow_mut().push(result.is_ok());
                    }
                })
            });
            owner.update(cx, |owner, cx| {
                owner.set_catalog_for_test(crate::pi_resources::Catalog {
                    root: root.path().to_owned(),
                    ..Default::default()
                });
                owner.change(
                    Change::Package {
                        command: command.clone(),
                        action: "install",
                        source: "npm:fixture".into(),
                    },
                    cx,
                );
            });
            let started = root.path().join("started");
            let wait = cx.update(|cx| {
                Tokio::spawn(cx, async move {
                    tokio::time::timeout(std::time::Duration::from_secs(5), async {
                        while !started.exists() {
                            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                        }
                    })
                    .await
                    .unwrap();
                })
            });
            wait.await.unwrap();
            owner.update(cx, |owner, cx| {
                owner.shutdown();
                assert!(owner.mutation().is_running());
                assert!(!owner.catalog().is_running());
                owner.refresh(cx);
                owner.change(Change::RegisterSkill(root.path().join("not-started")), cx);
                assert!(owner.mutation().package_running("npm:fixture", "install"));
            });
            std::fs::write(root.path().join("release"), "").unwrap();
            cx.condition(&owner, |owner, _| !owner.mutation().is_running())
                .await;
            assert!(root.path().join("finished").exists());
            assert_eq!(*results.borrow(), vec![!fail]);
            owner.update(cx, |owner, cx| {
                assert!(
                    !owner.catalog().is_running(),
                    "completion must not refresh during shutdown"
                );
                owner.refresh(cx);
                owner.change(
                    Change::Package {
                        command: command.clone(),
                        action: "remove",
                        source: "npm:fixture".into(),
                    },
                    cx,
                );
                assert!(!owner.busy());
            });
        }
        let root = tempfile::tempdir().unwrap();
        let reading =
            cx.new(|_| ResourceController::for_target(Target::Project(root.path().to_owned())));
        reading.update(cx, |owner, cx| {
            owner.refresh(cx);
            assert!(owner.catalog().is_running());
            owner.shutdown();
            assert!(!owner.catalog().is_running());
            owner.refresh(cx);
            assert!(!owner.catalog().is_running());
        });
    }
}
