//! Pi `settings.json` pages: scoped drafts, page saves, trust and models.
//!
//! The files on disk stay authoritative. Drafts live here until a page is
//! saved; each save re-reads the file inside Pi's lock and writes only the
//! changed leaves.
mod advanced;
mod advanced_controls;
mod fields;
mod forms;
mod inputs;
mod lists;
mod map_controls;
mod model_maps;
mod model_scope;
mod page;
mod resource_paths;

pub(super) use fields::Page;
pub(super) use page::request_scope;

use fields::Field;
use fields::Files;
use forms::{ConversationDraft, NetworkDraft, Slot, Snapshot, ToolsDraft};
use gpui_form::{
    DraftPolicy, Form, FormEvent, FormSchema, FormVersion, LifecycleError, ModelChangeKind,
};
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
use std::ffi::OsString;
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

#[derive(Clone, PartialEq)]
struct ScopeItem {
    value: Option<Scope>,
    title: SharedString,
}
impl SearchableListItem for ScopeItem {
    type Value = Option<Scope>;
    fn title(&self) -> SharedString {
        self.title.clone()
    }
    fn value(&self) -> &Self::Value {
        &self.value
    }
}

pub(super) enum PiConfigEvent {
    ResourcePathsSaved(Scope),
}
impl EventEmitter<PiConfigEvent> for PiConfig {}

pub(super) struct PiConfig {
    applied_pi: Entity<PiProbeController>,
    pub(super) file_pages: Entity<super::config_files::FilePages>,
    agent: Result<PathBuf, String>,
    scope: Scope,
    scope_select: Entity<SelectState<SearchableVec<ScopeItem>>>,
    global_select: Entity<SelectState<SearchableVec<ScopeItem>>>,
    scope_items: Vec<ScopeItem>,
    /// Projects opened or chosen in this session, offered by the scope menu.
    projects: Vec<PathBuf>,
    /// The project of the conversation shown when settings opened, listed first.
    active_project: Option<PathBuf>,
    /// Bumped whenever results for an earlier scope or read must be ignored.
    generation: u64,
    files: Load,
    trust: Option<Trust>,
    conversation: Entity<Form<ConversationDraft>>,
    network: Entity<Form<NetworkDraft>>,
    tools: Entity<Form<ToolsDraft>>,
    scope_loaded: bool,
    pending_reload: Option<PendingReload>,
    refresh_conflicts: Vec<Field>,
    saved_receipt: Option<SaveReceipt>,
    inputs: BTreeMap<Field, inputs::Adapter>,
    custom_tool: Entity<gpui_kit::component::input::InputState>,
    custom_tool_open: bool,
    resource_inputs: BTreeMap<Field, resource_paths::EntryInput>,
    conflicts: Vec<Field>,
    conflict_leaves: Vec<pi_settings::Leaf>,
    map_inputs: std::collections::HashMap<map_controls::MapInputKey, map_controls::MapAdapter>,
    error: Option<String>,
    saving: Option<Task<()>>,
    leave_after_save: Option<AfterSave>,
    trusting: Option<Task<()>>,
    models: Models,
    model_picker: Entity<SelectState<SearchableVec<ModelItem>>>,
    scope_picker: Entity<SelectState<SearchableVec<ModelItem>>>,
    map_pickers: BTreeMap<Field, Entity<SelectState<SearchableVec<ModelItem>>>>,
    map_picker_items: BTreeMap<Field, Vec<ModelItem>>,
    rows: std::collections::HashMap<model_scope::RowKey, model_scope::Adapter>,
    /// The model key last projected into `model_picker`.
    synced_model: Option<Option<(String, String)>>,
    file_task: Option<Task<()>>,
    reading_files: bool,
    resource_refresh_pending: bool,
    model_task: Option<Task<()>>,
    model_instance: Option<gupi_pi_runtime::InstanceId>,
    _subscriptions: Vec<Subscription>,
}

/// A completed disk write whose submitted sessions could not accept the result.
/// It is a single-save receipt, never an alternative editable baseline.
struct SaveReceipt {
    document: pi_settings::Document,
    /// The exact prepared models and their session-bound save credentials.
    submitted: (Snapshot, [Option<FormVersion>; 3]),
    installed: [bool; 3],
}

struct PendingReload {
    generation: u64,
    initial: bool,
    files: Files,
    trust: Option<Trust>,
    installed: [bool; 3],
}

impl PiConfig {
    pub(super) fn new(
        applied_pi: Entity<PiProbeController>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let initial_scope = ScopeItem {
            value: Some(Scope::Global),
            title: t(cx, "pi-scope-global").into(),
        };
        let scope_select = cx.new(|cx| {
            SelectState::new(
                SearchableVec::new(vec![initial_scope.clone()]),
                Some(gpui_kit::component::IndexPath::default()),
                window,
                cx,
            )
        });
        let global_select = cx.new(|cx| {
            SelectState::new(
                SearchableVec::new(vec![initial_scope]),
                Some(gpui_kit::component::IndexPath::default()),
                window,
                cx,
            )
        });
        let scope_select_sub =
            cx.subscribe_in(&scope_select, window, |this, picker, event, window, cx| {
                if let SelectEvent::<SearchableVec<ScopeItem>>::Confirm(Some(scope)) = event {
                    // A selection is a request. The leave guard can keep the current scope.
                    picker.update(cx, |picker, cx| {
                        picker.set_selected_value(&Some(this.scope.clone()), window, cx)
                    });
                    let entity = cx.entity();
                    let next = scope.clone();
                    window.defer(cx, move |window, cx| {
                        if let Some(next) = next {
                            request_scope(&entity, next, window, cx);
                        } else {
                            page::choose_project_folder(&entity, window, cx);
                        }
                    });
                }
            });
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
            this.set_values(Field::Model, value.into_iter().map(Slot::Set).collect(), cx);
        });
        let scope_picker = cx.new(|cx| {
            SelectState::new(SearchableVec::new(vec![]), None, window, cx).searchable(true)
        });
        let scope_sub =
            cx.subscribe_in(&scope_picker, window, |this, picker, event, window, cx| {
                if let SelectEvent::<SearchableVec<ModelItem>>::Confirm(Some((provider, model))) =
                    event
                {
                    this.append_pattern(format!("{provider}/{model}"), window, cx);
                    picker.update(cx, |picker, cx| picker.set_selected_index(None, window, cx));
                }
            });
        let conversation = cx.new(|_| Form::new(ConversationDraft::default()));
        let network = cx.new(|_| Form::new(NetworkDraft::default()));
        let tools = cx.new(|_| Form::new(ToolsDraft::default()));
        let tools_sub = cx.subscribe(&tools, |this, _, event, cx| this.form_changed(event, cx));
        let conversation_sub = cx.subscribe(&conversation, |this, _, event, cx| {
            this.form_changed(event, cx);
            if let FormEvent::ModelChanged(change) = event
                && (change.affects(&ConversationDraft::RESERVE_TOKENS)
                    || change.affects(&ConversationDraft::KEEP_RECENT_TOKENS))
            {
                // Token inheritance includes unsaved general values. The binding
                // refresh updates context without replacing drafts or pending input.
                this.refresh_map_inputs(cx);
            }
        });
        let network_sub = cx.subscribe(&network, |this, _, event, cx| this.form_changed(event, cx));
        let applied_sub = cx.observe(&applied_pi, |_, _, cx| cx.notify());
        let mut map_pickers = BTreeMap::new();
        let mut map_subscriptions = Vec::new();
        for field in [Field::ModelThinking, Field::ModelOverrides] {
            let picker = cx.new(|cx| {
                SelectState::new(SearchableVec::new(vec![]), None, window, cx).searchable(true)
            });
            map_subscriptions.push(cx.subscribe_in(
                &picker,
                window,
                move |this, picker, event, window, cx| {
                    if let SelectEvent::<SearchableVec<ModelItem>>::Confirm(Some((provider, id))) =
                        event
                    {
                        this.add_map_model(field, &format!("{provider}/{id}"), cx);
                        picker.update(cx, |picker, cx| picker.set_selected_index(None, window, cx));
                    }
                },
            ));
            map_pickers.insert(field, picker);
        }
        let custom_tool = cx.new(|cx| gpui_kit::component::input::InputState::new(window, cx));
        let custom_tool_sub =
            cx.subscribe_in(&custom_tool, window, |this, _, event, window, cx| {
                if matches!(
                    event,
                    gpui_kit::component::input::InputEvent::PressEnter { .. }
                ) {
                    this.finish_custom_tool(window, cx);
                }
            });
        let file_pages = cx.new(super::config_files::FilePages::new);
        let file_pages_sub = cx.observe(&file_pages, |_, _, cx| cx.notify());
        let mut subscriptions = vec![
            scope_select_sub,
            file_pages_sub,
            custom_tool_sub,
            picker_sub,
            scope_sub,
            applied_sub,
            conversation_sub,
            network_sub,
            tools_sub,
        ];
        subscriptions.extend(map_subscriptions);
        cx.on_release(|this, cx| this.cancel_model_query(cx))
            .detach();
        Self {
            file_pages,
            applied_pi,
            agent: pi_resources::agent_dir().map_err(|e| e.to_string()),
            scope: Scope::Global,
            scope_select,
            global_select,
            scope_items: Vec::new(),
            projects: Vec::new(),
            active_project: None,
            generation: 0,
            files: Load::Loading,
            trust: None,
            conversation,
            network,
            tools,
            scope_loaded: false,
            pending_reload: None,
            refresh_conflicts: Vec::new(),
            saved_receipt: None,
            inputs: BTreeMap::new(),
            custom_tool,
            custom_tool_open: false,
            resource_inputs: BTreeMap::new(),
            conflicts: Vec::new(),
            conflict_leaves: Vec::new(),
            map_inputs: std::collections::HashMap::new(),
            error: None,
            saving: None,
            leave_after_save: None,
            trusting: None,
            models: Models::Idle,
            model_picker,
            scope_picker,
            map_pickers,
            map_picker_items: BTreeMap::new(),
            rows: std::collections::HashMap::new(),
            synced_model: None,
            file_task: None,
            reading_files: false,
            resource_refresh_pending: false,
            model_task: None,
            model_instance: None,
            _subscriptions: subscriptions,
        }
    }

    // Semantic Form events update page feedback only; bindings own native projection.
    fn form_changed<M: FormSchema>(&mut self, event: &FormEvent<M>, cx: &mut Context<Self>) {
        if self.saved_receipt.is_none()
            && matches!(event,FormEvent::ModelChanged(change)|FormEvent::DraftChanged(change) if change.kind()==ModelChangeKind::Edit)
        {
            self.error = None;
        }
        cx.notify();
    }

    #[cfg(test)]
    pub(super) fn set_agent_for_test(&mut self, agent: PathBuf, cx: &mut App) {
        self.file_pages
            .update(cx, |files, _| files.set_agent_for_test(agent.clone()));
        self.agent = Ok(agent);
    }

    /// Re-reads the files when the Pi area is shown, unless drafts are open.
    pub(super) fn activate(&mut self, cx: &mut Context<Self>) {
        self.file_pages.update(cx, |files, cx| files.activate(cx));
        if !self.has_unsaved(cx) && !self.is_saving() {
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

    pub(super) fn has_unsaved(&self, cx: &App) -> bool {
        self.saved_receipt.is_some()
            || self.conversation.read(cx).is_dirty()
            || self.network.read(cx).is_dirty()
            || self.tools.read(cx).is_dirty()
            || self.file_pages.read(cx).dirty()
    }

    pub(super) fn needs_leave_guard(&self, cx: &App) -> bool {
        self.has_unsaved(cx) || self.is_saving() || self.file_pages.read(cx).busy()
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
        self.file_pages
            .update(cx, |files, cx| files.set_scope(scope.clone(), cx));
        self.scope = scope;
        self.scope_loaded = false;
        self.conversation.update(cx, |form, cx| {
            form.rebase(ConversationDraft::default(), DraftPolicy::Discard, cx)
                .expect("confirmed scope switch")
        });
        self.network.update(cx, |form, cx| {
            form.rebase(NetworkDraft::default(), DraftPolicy::Discard, cx)
                .expect("confirmed scope switch")
        });
        self.custom_tool_open = false;
        self.resource_inputs.clear();
        self.tools.update(cx, |form, cx| {
            form.rebase(ToolsDraft::default(), DraftPolicy::Discard, cx)
                .expect("confirmed scope switch")
        });
        self.pending_reload = None;
        self.saved_receipt = None;
        self.refresh_conflicts.clear();
        self.inputs.clear();
        self.rows.clear();
        self.map_inputs.clear();
        self.models = Models::Idle;
        self.map_picker_items.clear();
        self.reload(cx);
    }

    /// Re-reads the files. Drafts are kept so the user can compare and save
    /// again after a conflict.
    pub(super) fn reload(&mut self, cx: &mut Context<Self>) {
        if self.saved_receipt.is_some() {
            self.error = Some(t(cx, "pi-settings-newer-edits"));
            cx.notify();
            return;
        }
        self.reload_files(false, cx);
    }

    pub(super) fn refresh_resource_paths(&mut self, cx: &mut Context<Self>) {
        if self.is_saving() || self.saved_receipt.is_some() {
            self.resource_refresh_pending = true;
            return;
        }
        self.reload_files(true, cx);
    }

    fn reload_files(&mut self, preserve_arrays: bool, cx: &mut Context<Self>) {
        if self.is_saving() {
            return;
        }
        self.generation += 1;
        self.custom_tool_open = false;
        self.error = None;
        self.pending_reload = None;
        let previous = match &self.files {
            Load::Ready(files) if preserve_arrays => Some(files.clone()),
            _ => None,
        };
        self.reading_files = true;
        if !preserve_arrays {
            self.resource_refresh_pending = false;
            if !matches!(self.files, Load::Ready(_)) {
                self.files = Load::Loading;
                self.trust = None;
            }
        }
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
                this.reading_files = false;
                match result {
                    Ok((mut files, trust)) => {
                        if let Some(previous) = &previous {
                            let dirty = Field::RESOURCE_PATHS
                                .into_iter()
                                .filter(|f| this.pending(*f, cx))
                                .collect::<Vec<_>>();
                            files.preserve_dirty_resource_bases(previous, &dirty);
                        }
                        this.pending_reload = Some(PendingReload {
                            generation: this.generation,
                            initial: !this.scope_loaded,
                            files,
                            trust,
                            installed: [false; 3],
                        });
                        this.apply_pending_reload(cx);
                    }
                    Err(error) if matches!(this.files, Load::Ready(_)) => this.error = Some(error),
                    Err(error) => this.files = Load::Failed(error),
                }
                cx.notify();
            });
        }));
        cx.notify();
    }

    fn snapshot(&self, cx: &App) -> Snapshot {
        Snapshot::read(&self.conversation, &self.network, &self.tools, cx)
    }

    fn set_values(&mut self, field: Field, values: Vec<Slot<Value>>, cx: &mut Context<Self>) {
        if !matches!(self.files, Load::Ready(_)) {
            return;
        }
        let mut snapshot = self.snapshot(cx);
        snapshot.set(field, values);
        if let Err(error) = self.discard_field_drafts(field, cx).and_then(|_| {
            snapshot
                .install_field(field, &self.conversation, &self.network, &self.tools, cx)
                .map_err(|e| e.to_string())
        }) {
            self.error = Some(error.to_string());
            cx.notify();
            return;
        }
        self.retire_advanced_controls(field);
        self.error = None;
        cx.notify();
    }

    fn retire_advanced_controls(&mut self, field: Field) {
        if matches!(field, Field::DefaultTools | Field::NpmCommand) {
            self.custom_tool_open = false;
        }
        if field == Field::ModelOverrides {
            self.map_inputs.clear();
        }
    }

    fn invalid(&self, field: Field, cx: &App) -> bool {
        self.field_errors(field, cx)
            || self
                .snapshot(cx)
                .business_invalid(field, &self.baseline(cx))
    }

    fn resolved(&self, field: Field, cx: &App) -> fields::Resolved {
        let Load::Ready(files) = &self.files else {
            unreachable!("resolve loaded settings")
        };
        files.resolve(field, Some(&self.snapshot(cx).value(field)))
    }

    fn has_page_drafts(&self, page: Page, cx: &App) -> bool {
        let saved_pending = self.saved_receipt.as_ref().is_some_and(|receipt| {
            let ix = match page {
                Page::Conversation => 0,
                Page::Network => 1,
                Page::Tools => 2,
            };
            receipt.submitted.1[ix].is_some() && !receipt.installed[ix]
        });
        saved_pending
            || match page {
                Page::Conversation => self.conversation.read(cx).is_dirty(),
                Page::Network => self.network.read(cx).is_dirty(),
                Page::Tools => self.tools.read(cx).is_dirty(),
            }
    }

    pub(super) fn discard(&mut self, page: Option<Page>, cx: &mut Context<Self>) {
        if self.is_saving() {
            return;
        }
        if page.is_none_or(|p| p == Page::Conversation) {
            self.conversation.update(cx, |f, cx| {
                f.reset(DraftPolicy::Discard, cx)
                    .expect("explicit page discard")
            });
            self.rows.clear();
            self.map_inputs.clear();
        }
        if page.is_none_or(|p| p == Page::Network) {
            self.network.update(cx, |f, cx| {
                f.reset(DraftPolicy::Discard, cx)
                    .expect("explicit page discard")
            });
        }
        if page.is_none_or(|p| p == Page::Tools) {
            self.custom_tool_open = false;
            self.tools.update(cx, |f, cx| {
                f.reset(DraftPolicy::Discard, cx)
                    .expect("explicit page discard")
            });
        }
        self.refresh_conflicts
            .retain(|f| page.is_some_and(|p| f.page() != p));
        self.conflicts
            .retain(|f| page.is_some_and(|p| f.page() != p));
        self.conflict_leaves.retain(|leaf| {
            Field::ALL.iter().any(|f| {
                page.is_some_and(|p| f.page() != p)
                    && f.leaves()
                        .iter()
                        .any(|l| leaf.iter().map(String::as_str).eq(l.iter().copied()))
            })
        });
        self.error = None;
        if let Some(mut receipt) = self.saved_receipt.take() {
            let saved = Snapshot::project(&receipt.document);
            macro_rules! abandon_saved {
                ($ix:expr,$page:ident,$form:ident,$value:expr) => {
                    if page.is_none_or(|p| p == Page::$page) && receipt.submitted.1[$ix].is_some() {
                        self.$form
                            .update(cx, |f, cx| f.rebase($value, DraftPolicy::Discard, cx))
                            .expect("explicit discard of the saved page");
                        receipt.installed[$ix] = true;
                    }
                };
            }
            abandon_saved!(0, Conversation, conversation, saved.conversation);
            abandon_saved!(1, Network, network, saved.network);
            abandon_saved!(2, Tools, tools, saved.tools);
            if receipt
                .submitted
                .1
                .iter()
                .zip(receipt.installed)
                .all(|(v, installed)| v.is_none() || installed)
            {
                if let Load::Ready(files) = &self.files {
                    let mut files = files.clone();
                    match &mut files.project {
                        Some(project) => *project = receipt.document,
                        None => files.global = receipt.document,
                    }
                    self.pending_reload = Some(PendingReload {
                        generation: self.generation,
                        initial: false,
                        files,
                        trust: self.trust.clone(),
                        installed: receipt.installed,
                    });
                    self.apply_pending_reload(cx);
                }
            } else {
                self.saved_receipt = Some(receipt);
                self.error = Some(t(cx, "pi-settings-newer-edits"));
            }
        }
        self.refresh_inputs(cx);
        cx.notify();
    }

    fn apply_pending_reload(&mut self, cx: &mut Context<Self>) {
        let Some(mut pending) = self.pending_reload.take() else {
            return;
        };
        if pending.generation != self.generation {
            return;
        }
        let next = Snapshot::project(pending.files.edited());
        macro_rules! install {
            ($index:expr,$form:ident,$value:expr) => {
                if !pending.installed[$index] {
                    let result = self
                        .$form
                        .update(cx, |form, cx| if pending.initial { form.rebase($value, DraftPolicy::Discard, cx) } else { form.rebase_preserving_edits($value, cx) });
                    match result {
                        Ok(report) => {
                            debug_assert!(self.$form.read(cx).is_current(report.version()));
                            for field in self.conflict_fields(report.conflicts(), cx) {
                                if !self.refresh_conflicts.contains(&field) {
                                    self.refresh_conflicts.push(field);
                                }
                            }
                            pending.installed[$index] = true;
                        }
                        Err(LifecycleError::EditsPending) => {
                            self.pending_reload = Some(pending);
                            let this = cx.entity().downgrade();
                            cx.defer(move |cx| {
                                let _ = this.update(cx, |this, cx| this.apply_pending_reload(cx));
                            });
                            return;
                        }
                        Err(error) => {
                            tracing::warn!(%error, "Pi settings refresh rejected; edits retained");
                            self.error = Some(t(cx,"pi-settings-refresh-rejected"));
                            self.pending_reload = Some(pending);
                            cx.notify();
                            return;
                        }
                    }
                }
            };
        }
        install!(0, conversation, next.conversation);
        install!(1, network, next.network);
        install!(2, tools, next.tools);
        self.scope_loaded = true;
        self.files = Load::Ready(pending.files);
        self.trust = pending.trust;
        self.saved_receipt = None;
        self.refresh_inputs(cx);
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
        if self.saved_receipt.is_some() {
            self.error = Some(t(cx, "pi-settings-newer-edits"));
            cx.notify();
            return;
        }
        if self.is_saving() || self.pending_reload.is_some() {
            return;
        }
        let mut versions: [Option<FormVersion>; 3] = [None; 3];
        let baseline = self.baseline(cx);
        let mut prepared = self.snapshot(cx);
        if Field::ALL.into_iter().any(|field| {
            page.is_none_or(|p| field.page() == p) && prepared.business_invalid(field, &baseline)
        }) {
            self.error = Some(t(cx, "pi-settings-invalid"));
            cx.notify();
            return;
        }
        if page.is_none_or(|p| p == Page::Conversation) {
            let result = self.conversation.update(cx, |form, cx| form.prepare(cx));
            let Ok(snapshot) = result else {
                self.error = Some(t(cx, "pi-settings-invalid"));
                cx.notify();
                return;
            };
            versions[0] = Some(snapshot.version());
            prepared.conversation = snapshot.into_parts().1;
        }
        if page.is_none_or(|p| p == Page::Network) {
            let result = self.network.update(cx, |form, cx| form.prepare(cx));
            let Ok(snapshot) = result else {
                self.error = Some(t(cx, "pi-settings-invalid"));
                cx.notify();
                return;
            };
            versions[1] = Some(snapshot.version());
            prepared.network = snapshot.into_parts().1;
        }
        if page.is_none_or(|p| p == Page::Tools) {
            let result = self.tools.update(cx, |form, cx| form.prepare(cx));
            let Ok(snapshot) = result else {
                self.error = Some(t(cx, "pi-settings-invalid"));
                cx.notify();
                return;
            };
            versions[2] = Some(snapshot.version());
            prepared.tools = snapshot.into_parts().1;
        }
        let changes = files.changes(&prepared, &baseline, page);
        let paths_saved = changes.iter().any(|change| {
            Field::RESOURCE_PATHS
                .iter()
                .any(|field| change.leaf() == field.leaves()[0])
        });
        // A read started by a switch must not install a pre-save document afterwards.
        if self.reading_files {
            self.generation += 1;
            self.file_task = None;
            self.reading_files = false;
            self.resource_refresh_pending = true;
        }
        let scope = self.scope.clone();
        let agent = agent.clone();
        let generation = self.generation;
        let task = cx.background_spawn(async move { scope.commit(&agent, &changes) });
        self.error = None;
        self.conflicts.clear();
        self.conflict_leaves.clear();
        self.saving = Some(cx.spawn_in(window, async move |this, cx| {
            let result = task.await;
            let next = this.update(cx, |this, cx| {
                this.saving = None;
                cx.notify();
                if this.generation != generation {
                    this.leave_after_save = None;
                    return None;
                }
                if std::mem::take(&mut this.resource_refresh_pending) {
                    let config = cx.entity().downgrade();
                    cx.defer(move |cx| {
                        let _ = config.update(cx, |this, cx| this.refresh_resource_paths(cx));
                    });
                }
                match result {
                    Ok(mut document) => {
                        // Preserve the actual disk result before projecting retained resource bases.
                        let saved_document = document.clone();
                        if page.is_some_and(|p| p != Page::Tools)
                            && let Load::Ready(previous) = &this.files
                        {
                            let mut next_files = previous.clone();
                            match &mut next_files.project {
                                Some(project) => *project = document,
                                None => next_files.global = document,
                            }
                            let dirty = Field::RESOURCE_PATHS
                                .into_iter()
                                .filter(|f| this.pending(*f, cx))
                                .collect::<Vec<_>>();
                            next_files.preserve_dirty_resource_bases(previous, &dirty);
                            document = next_files.edited().clone();
                        }
                        let next = Snapshot::project(&document);
                        let current = versions[0]
                            .is_none_or(|v| this.conversation.read(cx).is_current(v))
                            && versions[1].is_none_or(|v| this.network.read(cx).is_current(v))
                            && versions[2].is_none_or(|v| this.tools.read(cx).is_current(v));
                        if !current {
                            this.saved_receipt = Some(SaveReceipt {
                                document: saved_document,
                                submitted: (prepared, versions),
                                installed: [false; 3],
                            });
                            this.error = Some(t(cx, "pi-settings-newer-edits"));
                            this.leave_after_save = None;
                            return None;
                        }
                        // No native callback can run between the checks and these installations.
                        if let Some(version) = versions[0] {
                            assert!(this.conversation.update(cx, |f, cx| f.rebase_if_current(
                                version,
                                next.conversation,
                                cx
                            )));
                        }
                        if let Some(version) = versions[1] {
                            assert!(this.network.update(cx, |f, cx| f.rebase_if_current(
                                version,
                                next.network,
                                cx
                            )));
                        }
                        if let Some(version) = versions[2] {
                            assert!(
                                this.tools.update(cx, |f, cx| f
                                    .rebase_if_current(version, next.tools, cx))
                            );
                        }
                        let Load::Ready(files) = &this.files else {
                            unreachable!("saved loaded files")
                        };
                        let mut files = files.clone();
                        match &mut files.project {
                            Some(project) => *project = document,
                            None => files.global = document,
                        }
                        this.pending_reload = Some(PendingReload {
                            generation: this.generation,
                            initial: !this.scope_loaded,
                            files,
                            trust: this.trust.clone(),
                            installed: versions.map(|v| v.is_some()),
                        });
                        this.apply_pending_reload(cx);
                        if this.pending_reload.is_some() {
                            this.leave_after_save = None;
                            return None;
                        }
                        this.refresh_conflicts
                            .retain(|f| page.is_some_and(|p| f.page() != p));
                        if paths_saved {
                            cx.emit(PiConfigEvent::ResourcePathsSaved(this.scope.clone()));
                        }
                        return this.leave_after_save.take().or(then);
                    }
                    Err(pi_settings::Error::Conflict(leaves)) => {
                        this.conflict_leaves = leaves.clone();
                        this.conflicts = Field::ALL
                            .into_iter()
                            .filter(|field| {
                                field.leaves().iter().any(|l| {
                                    leaves.iter().any(|leaf| {
                                        leaf.len() >= l.len()
                                            && leaf
                                                .iter()
                                                .take(l.len())
                                                .map(String::as_str)
                                                .eq(l.iter().copied())
                                    })
                                })
                            })
                            .collect();
                        this.error = Some(t(cx, "pi-settings-conflict"));
                    }
                    Err(pi_settings::Error::Locked(_)) => {
                        this.error = Some(t(cx, "pi-settings-locked"));
                    }
                    Err(error) => this.error = Some(error.to_string()),
                }
                this.leave_after_save = None;
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
        let files = this.read(cx).file_pages.clone();
        let this = this.clone();
        super::config_files::FilePages::confirm_leave(
            &files,
            Box::new(move |window, cx| {
                Self::confirm_settings_leave(&this, proceed, window, cx);
            }),
            window,
            cx,
        );
    }

    fn confirm_settings_leave(
        this: &Entity<Self>,
        proceed: impl FnOnce(&mut Window, &mut App) + 'static,
        window: &mut Window,
        cx: &mut App,
    ) {
        if this.read(cx).is_saving() {
            let target = this.clone();
            this.update(cx, |this, _| {
                if this.leave_after_save.is_none() {
                    this.leave_after_save = Some(Box::new(move |window, cx| {
                        Self::confirm_settings_leave(&target, proceed, window, cx);
                    }));
                }
            });
            return;
        }
        if !this.read(cx).has_unsaved(cx) {
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
                                    if discard.0.read(cx).is_saving() {
                                        return;
                                    }
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
                .footer(super::dialog_buttons(
                    "pi-trust-confirm",
                    false,
                    false,
                    crate::features::settings::DialogCommit::Primary,
                    cx,
                ))
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

    pub(super) fn mcp_command(&self, cx: &App) -> Option<OsString> {
        self.applied_pi
            .read(cx)
            .data()
            .map(|data| data.command.as_os_str().to_owned())
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
                            picker.set_items(SearchableVec::new(items.clone()), window, cx)
                        });
                        this.scope_picker.update(cx, |picker, cx| {
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
