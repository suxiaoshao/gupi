use crate::pi_resources as io;
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
    },
    Delete(Resource),
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
    catalog: refresh::Operation<Catalog, Error, Task<()>>,
    mutation: Mutation,
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
        Self {
            catalog: refresh::Operation::new(),
            mutation: Mutation::Idle,
        }
    }
    pub fn busy(&self) -> bool {
        self.catalog.is_running() || self.mutation.is_running()
    }
    pub fn stop(&mut self) {
        self.catalog.transition(Cancel);
        self.mutation = Mutation::Idle;
    }
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.refresh_after(None, cx);
    }
    fn refresh_after(&mut self, change: Option<Change>, cx: &mut Context<Self>) {
        if self.busy() {
            return;
        }
        let previous = change.as_ref().and_then(|_| self.catalog.data().cloned());
        let environment = gupi_pi_runtime::environment(cx);
        let worker = Tokio::spawn(cx, async move {
            if let (Some(change), Some(mut catalog)) = (change, previous) {
                match change {
                    Change::Toggle(resource, enabled) => {
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
        if self.busy() {
            return;
        }
        let Some(root) = self.catalog.data().map(|c| c.root.clone()) else {
            return;
        };
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
                    io::package_action(command, root, action, source, snapshot.variables())
                        .await
                        .map_err(|error| Error(snapshot.explain(error)))
                }
                change => tokio::task::spawn_blocking(move || match change {
                    Change::Toggle(resource, active) => io::set_enabled(&root, &resource, active),
                    Change::RegisterSkill(path) => io::register_skill(&root, &path),
                    Change::Save { path, text, create } => io::save_text(&path, &text, create),
                    Change::Delete(resource) => {
                        if !resource.editable
                            || resource.package.is_some()
                            || !matches!(resource.kind, Kind::Skill | Kind::Prompt)
                        {
                            return Err(Error("Resource is read-only".into()));
                        }
                        trash::delete(&resource.path).map_err(|e| Error(e.to_string()))
                    }
                    Change::Package { .. } => unreachable!(),
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
            });
        });
        self.mutation = Mutation::Running {
            change: active,
            task,
        };
        cx.notify();
    }
}

#[cfg(feature = "test-support")]
impl ResourceController {
    pub fn set_catalog_for_test(&mut self, catalog: Catalog) {
        self.catalog.transition(Load(Task::ready(())));
        self.catalog.transition(Complete(Ok(catalog)));
    }
}

#[cfg(test)]
mod tests {
    use super::{Change, Error, Mutation, ResourceController};
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

    #[test]
    fn package_loading_matches_the_running_request_and_clears_on_stop() {
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
            owner.stop();
            assert!(!owner.mutation().is_running());
            assert!(!owner.mutation().package_running("npm:example", action));
        }
    }
}
