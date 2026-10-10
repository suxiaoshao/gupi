//! Pi `settings.json` pages: scoped drafts, page saves, trust and models.
//!
//! The files on disk stay authoritative. Drafts live here until a page is
//! saved; each save re-reads the file inside Pi's lock and writes only the
//! changed leaves.
mod fields;
mod page;

pub(super) use fields::Page;
pub(super) use page::request_scope;

use fields::Edit;
use fields::Field;
use fields::Files;
use gpui_kit::component::WindowExt as _;
use gpui_kit::component::searchable_list::SearchableListItem;
use gpui_kit::component::searchable_list::SearchableVec;
use gpui_kit::component::select::SelectEvent;
use gpui_kit::component::select::SelectState;
use gpui_kit::*;
use gupi_pi_runtime::PiProbeController;
use gupi_resources::pi_resources;
use gupi_resources::pi_settings;
use gupi_resources::pi_settings::Scope;
use gupi_resources::pi_settings::trust;
use gupi_settings::i18n::t;
use pi_rpc::LaunchOptions;
use pi_rpc::protocol::Model;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;

/// Runs after a successful save, e.g. to continue leaving the page.
type AfterSave = Box<dyn FnOnce(&mut Window, &mut App)>;

/// The oldest Pi whose settings and trust formats these pages implement.
const MINIMUM_PI: (u64, u64) = (1, 1);

enum Load {
    Loading,
    Ready(Files),
    Failed(String),
}

/// The stored trust decision for a project scope.
#[derive(Clone, Debug, PartialEq)]
enum Trust {
    Trusted,
    /// No record trusts the project and the global policy does not either.
    Untrusted,
    Unknown(String),
}

enum Models {
    Idle,
    Loading,
    Ready(Vec<Model>),
    Failed(String),
}

#[derive(Clone, PartialEq)]
pub(super) struct ModelItem {
    key: (String, String),
    title: SharedString,
}
impl SearchableListItem for ModelItem {
    type Value = (String, String);
    fn title(&self) -> SharedString {
        self.title.clone()
    }
    fn value(&self) -> &(String, String) {
        &self.key
    }
    fn matches(&self, query: &str) -> bool {
        let query = query.to_lowercase();
        self.title.to_lowercase().contains(&query)
            || self.key.0.to_lowercase().contains(&query)
            || self.key.1.to_lowercase().contains(&query)
    }
}

pub(super) struct PiConfig {
    applied_pi: Entity<PiProbeController>,
    agent: Result<PathBuf, String>,
    scope: Scope,
    /// Projects opened or chosen in this session, offered by the scope menu.
    projects: Vec<PathBuf>,
    /// The project of the conversation shown when settings opened, listed first.
    active_project: Option<PathBuf>,
    /// Bumped whenever results for an earlier scope or read must be ignored.
    generation: u64,
    files: Load,
    trust: Option<Trust>,
    drafts: BTreeMap<Field, Edit>,
    conflicts: Vec<Field>,
    error: Option<String>,
    saving: Option<Task<()>>,
    trusting: Option<Task<()>>,
    models: Models,
    model_picker: Entity<SelectState<SearchableVec<ModelItem>>>,
    /// The model key last projected into `model_picker`.
    synced_model: Option<Option<(String, String)>>,
    file_task: Option<Task<()>>,
    model_task: Option<Task<()>>,
    model_instance: Option<gupi_pi_runtime::InstanceId>,
    _subscriptions: Vec<Subscription>,
}

impl PiConfig {
    pub(super) fn new(
        applied_pi: Entity<PiProbeController>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let model_picker = cx.new(|cx| {
            SelectState::new(SearchableVec::new(vec![]), None, window, cx).searchable(true)
        });
        let picker_sub = cx.subscribe(&model_picker, |this, _, event, cx| {
            let SelectEvent::<SearchableVec<ModelItem>>::Confirm(Some((provider, model))) = event
            else {
                return;
            };
            let value = vec![
                Value::String(provider.clone()),
                Value::String(model.clone()),
            ];
            this.set_draft(Field::Model, Edit::Set(value), cx);
        });
        let applied_sub = cx.observe(&applied_pi, |_, _, cx| cx.notify());
        cx.on_release(|this, cx| this.cancel_model_query(cx))
            .detach();
        Self {
            applied_pi,
            agent: pi_resources::agent_dir().map_err(|e| e.to_string()),
            scope: Scope::Global,
            projects: Vec::new(),
            active_project: None,
            generation: 0,
            files: Load::Loading,
            trust: None,
            drafts: BTreeMap::new(),
            conflicts: Vec::new(),
            error: None,
            saving: None,
            trusting: None,
            models: Models::Idle,
            model_picker,
            synced_model: None,
            file_task: None,
            model_task: None,
            model_instance: None,
            _subscriptions: vec![picker_sub, applied_sub],
        }
    }

    #[cfg(test)]
    pub(super) fn set_agent_for_test(&mut self, agent: PathBuf) {
        self.agent = Ok(agent);
    }

    /// Re-reads the files when the Pi area is shown, unless drafts are open.
    pub(super) fn activate(&mut self, cx: &mut Context<Self>) {
        if !self.has_unsaved() && !self.is_saving() {
            self.reload(cx);
        }
    }

    pub(super) fn scope(&self) -> &Scope {
        &self.scope
    }

    /// The stored trust decision for the current project scope, if known.
    pub(super) fn trusted(&self) -> Option<bool> {
        match (&self.scope, &self.trust) {
            (Scope::Project(_), Some(Trust::Trusted)) => Some(true),
            (Scope::Project(_), Some(Trust::Untrusted)) => Some(false),
            _ => None,
        }
    }

    /// Remembers the current conversation's project for the scope menu.
    pub(super) fn set_active_project(&mut self, cwd: Option<PathBuf>, cx: &mut Context<Self>) {
        self.active_project = cwd
            .filter(|cwd| !cwd.as_os_str().is_empty())
            .map(|cwd| trust::canonical(&cwd));
        cx.notify();
    }

    /// Scope menu entries: global, the current conversation's project, then
    /// other projects from this session.
    fn scope_choices(&self) -> Vec<Scope> {
        let mut projects: Vec<PathBuf> = self.active_project.iter().cloned().collect();
        for project in &self.projects {
            if !projects.contains(project) {
                projects.push(project.clone());
            }
        }
        std::iter::once(Scope::Global)
            .chain(projects.into_iter().map(Scope::Project))
            .collect()
    }

    pub(super) fn has_unsaved(&self) -> bool {
        !self.drafts.is_empty()
    }

    fn is_saving(&self) -> bool {
        self.saving.is_some()
    }

    /// Whether the running Pi implements the formats these pages edit.
    fn support(&self, cx: &App) -> Support {
        let probe = self.applied_pi.read(cx);
        match probe.data() {
            Some(data) if supports(&data.version) => Support::Ready,
            Some(data) => Support::Unsupported(data.version.clone()),
            None if probe.is_running() => Support::Checking,
            None => Support::Unavailable,
        }
    }

    /// Switches scope, discarding drafts. Callers confirm unsaved changes first.
    pub(super) fn set_scope(&mut self, scope: Scope, cx: &mut Context<Self>) {
        let scope = match scope.validated() {
            Ok(scope) => scope,
            Err(error) => {
                self.error = Some(error.to_string());
                cx.notify();
                return;
            }
        };
        if let Scope::Project(cwd) = &scope
            && !self.projects.contains(cwd)
        {
            self.projects.push(cwd.clone());
        }
        if self.scope == scope {
            return;
        }
        self.cancel_model_query(cx);
        self.scope = scope;
        self.drafts.clear();
        self.models = Models::Idle;
        self.reload(cx);
    }

    /// Re-reads the files. Drafts are kept so the user can compare and save
    /// again after a conflict.
    pub(super) fn reload(&mut self, cx: &mut Context<Self>) {
        self.generation += 1;
        self.conflicts.clear();
        self.error = None;
        self.files = Load::Loading;
        self.trust = None;
        let generation = self.generation;
        let agent = self.agent.clone();
        let scope = self.scope.clone();
        let task = cx.background_spawn(async move {
            let agent = agent?;
            let global =
                pi_settings::load(&Scope::Global.file(&agent)).map_err(|e| e.to_string())?;
            let (project, trust) = match &scope {
                Scope::Global => (None, None),
                Scope::Project(cwd) => {
                    let project =
                        pi_settings::load(&scope.file(&agent)).map_err(|e| e.to_string())?;
                    (Some(project), Some(read_trust(&agent, cwd, &global)))
                }
            };
            Ok((Files { global, project }, trust))
        });
        self.file_task = Some(cx.spawn(async move |this, cx| {
            let result: Result<_, String> = task.await;
            let _ = this.update(cx, |this, cx| {
                if this.generation != generation {
                    return;
                }
                match result {
                    Ok((files, trust)) => {
                        this.files = Load::Ready(files);
                        this.trust = trust;
                    }
                    Err(error) => this.files = Load::Failed(error),
                }
                cx.notify();
            });
        }));
        cx.notify();
    }

    fn set_draft(&mut self, field: Field, edit: Edit, cx: &mut Context<Self>) {
        let Load::Ready(files) = &self.files else {
            return;
        };
        // An edit back to the stored value is no change at all.
        let stored = files.resolve(field, None);
        let unchanged = match &edit {
            Edit::Set(values) => stored.overridden && stored.values == wrap(values),
            Edit::Remove => !stored.overridden,
            Edit::Invalid(_) => false,
        };
        if unchanged {
            self.drafts.remove(&field);
        } else {
            self.drafts.insert(field, edit);
        }
        self.error = None;
        cx.notify();
    }

    fn clear_draft(&mut self, field: Field, cx: &mut Context<Self>) {
        self.drafts.remove(&field);
        cx.notify();
    }

    /// Fixes the current effective value in the edited scope.
    fn pin(&mut self, field: Field, cx: &mut Context<Self>) {
        let Load::Ready(files) = &self.files else {
            return;
        };
        let resolved = files.resolve(field, self.drafts.get(&field));
        if let Some(values) = resolved.values.into_iter().collect::<Option<Vec<_>>>() {
            self.drafts.insert(field, Edit::Set(values));
            cx.notify();
        }
    }

    fn page_drafts(&self, page: Option<Page>) -> BTreeMap<Field, Edit> {
        self.drafts
            .iter()
            .filter(|(field, _)| page.is_none_or(|page| field.page() == page))
            .map(|(field, edit)| (*field, edit.clone()))
            .collect()
    }

    fn has_page_drafts(&self, page: Page) -> bool {
        self.drafts.keys().any(|field| field.page() == page)
    }

    pub(super) fn discard(&mut self, page: Option<Page>, cx: &mut Context<Self>) {
        self.drafts
            .retain(|field, _| page.is_some_and(|page| field.page() != page));
        self.conflicts.clear();
        self.error = None;
        cx.notify();
    }

    /// Saves one page, or every page. `then` runs after a successful save.
    pub(super) fn save(
        &mut self,
        page: Option<Page>,
        then: Option<AfterSave>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (Load::Ready(files), Ok(agent)) = (&self.files, &self.agent) else {
            return;
        };
        if self.is_saving() {
            return;
        }
        let drafts = self.page_drafts(page);
        if drafts.values().any(|edit| matches!(edit, Edit::Invalid(_))) {
            self.error = Some(t(cx, "pi-settings-invalid"));
            cx.notify();
            return;
        }
        let changes = files.changes(&drafts);
        let scope = self.scope.clone();
        let agent = agent.clone();
        let generation = self.generation;
        let saved: Vec<Field> = drafts.keys().copied().collect();
        let task = cx.background_spawn(async move { scope.commit(&agent, &changes) });
        self.error = None;
        self.conflicts.clear();
        self.saving = Some(cx.spawn_in(window, async move |this, cx| {
            let result = task.await;
            let next = this.update(cx, |this, cx| {
                this.saving = None;
                cx.notify();
                if this.generation != generation {
                    return None;
                }
                match result {
                    Ok(document) => {
                        if let Load::Ready(files) = &mut this.files {
                            match &mut files.project {
                                Some(project) => *project = document,
                                None => files.global = document,
                            }
                        }
                        // Drafts typed while saving are newer than what was written.
                        for field in &saved {
                            if this.drafts.get(field) == drafts.get(field) {
                                this.drafts.remove(field);
                            }
                        }
                        return then;
                    }
                    Err(pi_settings::Error::Conflict(leaves)) => {
                        this.conflicts = Field::ALL
                            .into_iter()
                            .filter(|field| field.leaves().iter().any(|l| leaves.contains(l)))
                            .collect();
                        this.error = Some(t(cx, "pi-settings-conflict"));
                    }
                    Err(pi_settings::Error::Locked(_)) => {
                        this.error = Some(t(cx, "pi-settings-locked"));
                    }
                    Err(error) => this.error = Some(error.to_string()),
                }
                None
            });
            // The continuation may update this entity, so it runs afterwards.
            if let Ok(Some(next)) = next {
                let _ = cx.update(|window, cx| next(window, cx));
            }
        }));
        cx.notify();
    }

    /// Asks before discarding drafts, then runs `proceed`.
    pub(super) fn confirm_leave(
        this: &Entity<Self>,
        proceed: impl FnOnce(&mut Window, &mut App) + 'static,
        window: &mut Window,
        cx: &mut App,
    ) {
        if !this.read(cx).has_unsaved() {
            proceed(window, cx);
            return;
        }
        let this = this.clone();
        let proceed = std::rc::Rc::new(std::cell::RefCell::new(Some(
            Box::new(proceed) as AfterSave
        )));
        window.open_dialog(cx, move |dialog, _, cx| {
            let discard = (this.clone(), proceed.clone());
            let save = (this.clone(), proceed.clone());
            dialog
                .title(t(cx, "pi-settings-unsaved-title"))
                .child(t(cx, "pi-settings-unsaved-body"))
                .footer(
                    gpui_kit::component::dialog::DialogFooter::new()
                        .child(
                            gpui_kit::component::button::Button::new("pi-unsaved-cancel")
                                .label(t(cx, "action-cancel"))
                                .on_click(|_, window, cx| window.close_dialog(cx)),
                        )
                        .child(
                            gpui_kit::component::button::Button::new("pi-unsaved-discard")
                                .label(t(cx, "pi-settings-discard"))
                                .on_click(move |_, window, cx| {
                                    window.close_dialog(cx);
                                    discard.0.update(cx, |this, cx| this.discard(None, cx));
                                    if let Some(proceed) = discard.1.borrow_mut().take() {
                                        proceed(window, cx);
                                    }
                                }),
                        )
                        .child({
                            use gpui_kit::component::button::ButtonVariants as _;
                            gpui_kit::component::button::Button::new("pi-unsaved-save")
                                .primary()
                                .label(t(cx, "pi-settings-save"))
                                .on_click(move |_, window, cx| {
                                    window.close_dialog(cx);
                                    let proceed = save.1.borrow_mut().take();
                                    save.0.update(cx, |this, cx| {
                                        this.save(
                                            None,
                                            proceed.map(|p| {
                                                p as Box<dyn FnOnce(&mut Window, &mut App)>
                                            }),
                                            window,
                                            cx,
                                        )
                                    });
                                })
                        }),
                )
        });
    }

    /// Records trust for exactly the current project after confirmation.
    fn confirm_trust(this: &Entity<Self>, window: &mut Window, cx: &mut App) {
        let Scope::Project(cwd) = this.read(cx).scope.clone() else {
            return;
        };
        let folder = trust::canonical(&cwd);
        let name = folder
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| folder.display().to_string());
        let this = this.clone();
        window.open_dialog(cx, move |dialog, _, cx| {
            let mut args = fluent_bundle::FluentArgs::new();
            args.set("name", name.clone());
            let title = gupi_settings::i18n::t_with_args(cx, "pi-trust-title", &args);
            let this = this.clone();
            dialog
                .title(title)
                .child(
                    gpui_kit::component::v_flex()
                        .gap_2()
                        .child(div().text_sm().child(folder.display().to_string()))
                        .child(t(cx, "pi-trust-body")),
                )
                .footer(super::dialog_buttons("pi-trust-confirm", false, false, cx))
                .on_ok(move |_, window, cx| {
                    this.update(cx, |this, cx| this.trust(window, cx));
                    true
                })
        });
    }

    fn trust(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (Scope::Project(cwd), Ok(agent)) = (self.scope.clone(), self.agent.clone()) else {
            return;
        };
        let generation = self.generation;
        let task = cx.background_spawn(async move { trust::trust(&agent, &cwd) });
        self.trusting = Some(cx.spawn_in(window, async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                this.trusting = None;
                if this.generation != generation {
                    return;
                }
                match result {
                    Ok(_) => this.trust = Some(Trust::Trusted),
                    Err(error) => this.error = Some(error.to_string()),
                }
                cx.notify();
            });
        }));
        cx.notify();
    }

    fn cancel_model_query(&mut self, cx: &mut App) {
        if let Some(id) = self.model_instance.take() {
            gupi_pi_runtime::global(cx).update(cx, |pi, cx| pi.close(id, cx).detach());
        }
        self.model_task = None;
    }

    /// Lists models from a dedicated Pi with no session and no prompt. A
    /// project query runs in that project, so Pi applies its own trust rules.
    fn load_models(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(command) = self
            .applied_pi
            .read(cx)
            .data()
            .map(|data| data.command.clone())
        else {
            return;
        };
        // A global query must not pick up a project from the app's directory.
        let (cwd, scratch) = match &self.scope {
            Scope::Project(cwd) => (cwd.clone(), None),
            Scope::Global => match tempfile::tempdir() {
                Ok(dir) => (dir.path().to_owned(), Some(dir)),
                Err(error) => {
                    self.models = Models::Failed(error.to_string());
                    cx.notify();
                    return;
                }
            },
        };
        let mut options = LaunchOptions::new(command, cwd);
        options.args.push("--no-session".into());
        let pi = gupi_pi_runtime::global(cx);
        let id = match pi.update(cx, |pi, cx| pi.start(options, cx)) {
            Ok(id) => id,
            Err(error) => {
                self.models = Models::Failed(error.to_string());
                cx.notify();
                return;
            }
        };
        self.models = Models::Loading;
        self.model_instance = Some(id);
        self.model_task = Some(cx.spawn_in(window, async move |this, cx| {
            let result = query_models(&pi, id, cx).await;
            pi.update(cx, |pi, cx| pi.close(id, cx).detach());
            drop(scratch);
            let _ = this.update_in(cx, |this, window, cx| {
                if this.model_instance != Some(id) {
                    return;
                }
                this.model_instance = None;
                this.models = match result {
                    Ok(models) => {
                        let items: Vec<ModelItem> = models
                            .iter()
                            .map(|model| ModelItem {
                                key: (model.provider.clone(), model.id.clone()),
                                title: format!("{} · {}", model.name, model.provider).into(),
                            })
                            .collect();
                        this.model_picker.update(cx, |picker, cx| {
                            picker.set_items(SearchableVec::new(items), window, cx)
                        });
                        Models::Ready(models)
                    }
                    Err(error) => Models::Failed(error),
                };
                // New items: project the selection again.
                this.synced_model = None;
                cx.notify();
            });
        }));
        cx.notify();
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Support {
    Ready,
    Checking,
    Unavailable,
    Unsupported(String),
}

fn supports(version: &str) -> bool {
    let mut parts = version
        .trim()
        .trim_start_matches('v')
        .split(['.', '-', '+'])
        .map(|part| part.parse::<u64>());
    match (parts.next(), parts.next()) {
        (Some(Ok(major)), Some(Ok(minor))) => (major, minor) >= MINIMUM_PI,
        _ => false,
    }
}

fn wrap(values: &[Value]) -> Vec<Option<Value>> {
    values.iter().cloned().map(Some).collect()
}

fn read_trust(agent: &Path, cwd: &Path, global: &pi_settings::Document) -> Trust {
    match trust::decision(agent, cwd) {
        Ok(trust::Decision::Trusted(_)) => Trust::Trusted,
        Ok(trust::Decision::Distrusted(_)) => Trust::Untrusted,
        Ok(trust::Decision::Unset) => {
            match global.get(&["defaultProjectTrust"]).and_then(Value::as_str) {
                Some("always") => Trust::Trusted,
                _ => Trust::Untrusted,
            }
        }
        Err(error) => Trust::Unknown(error.to_string()),
    }
}

async fn query_models(
    pi: &Entity<gupi_pi_runtime::PiState>,
    id: gupi_pi_runtime::InstanceId,
    cx: &mut AsyncWindowContext,
) -> Result<Vec<Model>, String> {
    let start = std::time::Instant::now();
    let client = loop {
        let client = pi
            .read_with(cx, |pi, _| pi.client(id))
            .map_err(|e| e.to_string())?;
        if let Some(client) = client {
            break client;
        }
        if start.elapsed().as_secs() > 30 {
            return Err("Pi connection timed out".to_owned());
        }
        smol::Timer::after(std::time::Duration::from_millis(30)).await;
    };
    client.ready().await.map_err(|e| e.to_string())?;
    client
        .get_available_models()
        .await
        .map(|models| models.models)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
#[path = "pi_config/tests.rs"]
mod tests;
