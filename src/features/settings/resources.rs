use super::*;
use gpui_form::FormSchema;
use gpui_kit::component::ElementExt as _;
use gpui_kit::component::Sizable;
use gpui_kit::component::WindowExt;
use gpui_kit::component::input::Editor as CodeEditor;
use gpui_kit::component::input::EditorState;
use gpui_kit::component::menu::PopupMenu;
use gpui_kit::component::menu::PopupMenuItem;
use gpui_kit::component::notification::Notification;
use gpui_kit::component::switch::Switch;
use gpui_kit::component::tag::Tag;
use gpui_kit::prelude::FluentBuilder;
use gpui_operation::Complete;
use gpui_operation::Load;
use gpui_operation::Refresh;
use gpui_operation::Retry;
use gpui_operation::Transition;
use gpui_operation::refresh;
use gupi_resources::pi_resources as io;
use gupi_resources::pi_resources::Baseline;
use gupi_resources::pi_resources::Kind;
use gupi_resources::pi_resources::Resource;
use gupi_resources::pi_settings::Scope;
use gupi_resources::resources::Change;
use gupi_resources::resources::ResourceController;
use gupi_resources::resources::ResourceEvent;
use gupi_resources::resources::Target;
use std::path::PathBuf;

mod browse;
mod editor;
mod packages;
mod prompts;
mod skills;

#[derive(Clone, Default, PartialEq, FormSchema)]
struct TextDraft {
    text: String,
}
struct Editor {
    path: PathBuf,
    kind: Kind,
    editable: bool,
    create: bool,
    /// The controller whose files this editor writes, fixed when opened.
    owner: Entity<ResourceController>,
    /// The contents read when opened, checked again before saving.
    baseline: Baseline,
    /// The file changed on disk after opening; saving was refused.
    changed: bool,
    form: Entity<Form<TextDraft>>,
    input: Entity<EditorState>,
    _binding: ControlBinding,
    _subscriptions: Vec<Subscription>,
}
#[derive(Clone)]
struct Open {
    path: PathBuf,
    kind: Kind,
    editable: bool,
    create: bool,
    owner: Entity<ResourceController>,
}

/// How a skill or prompt row appears in the current scope.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Row {
    /// Global scope: the full personal resource editor.
    Global,
    /// Project scope: a file from the project's own `.pi` folder.
    Own,
    /// Project scope: a global resource, read-only here.
    Inherited,
}

/// Runs once unsaved resource text is saved or discarded.
type Proceed = Box<dyn FnOnce(&mut Window, &mut App)>;
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Section {
    Overview,
    Packages,
    Catalog,
}
/// Requests that settings show another page.
pub(super) enum Navigate {
    /// The installed packages view, with the requested package expanded.
    Package,
    Pi,
}

struct ProjectResources {
    controller: Entity<ResourceController>,
    _subscriptions: Vec<Subscription>,
}

pub(super) struct ResourcesView {
    /// Global resources and packages; always loaded.
    pub controller: Entity<ResourceController>,
    /// The selected project's own text resources, when the Pi scope is a project.
    project: Option<Entity<ResourceController>>,
    /// The shared Pi scope; skills and prompts follow it.
    pi_config: Option<Entity<super::pi_config::PiConfig>>,
    /// Continues a leave request once the editor's save succeeds.
    after_save: Option<(PathBuf, Proceed)>,
    projects: std::collections::BTreeMap<PathBuf, ProjectResources>,
    config: Entity<ConfigController>,
    applied_pi: Entity<PiProbeController>,
    source: Entity<InputState>,
    names: [Entity<InputState>; 2],
    search: Entity<InputState>,
    prompt_search: Entity<InputState>,
    previews: std::collections::BTreeMap<PathBuf, refresh::Operation<String, io::Error, Task<()>>>,
    editor: Option<Editor>,
    open: refresh::Operation<(), io::Error, Task<()>>,
    creating: Option<Kind>,
    installing: bool,
    expanded_packages: std::collections::BTreeSet<String>,
    /// Packages page view: the catalog when true, installed packages otherwise.
    browsing: bool,
    browse: browse::Browse,
    installed_scroll: ScrollHandle,
    /// The Packages body's width from the last layout; drives its layout choices.
    packages_width: Option<Pixels>,
    /// The latest Pi operation result per registry package name, shown in details.
    package_results: std::collections::BTreeMap<String, Result<(), io::Error>>,
    error: Option<(Kind, String)>,
    _subscriptions: Vec<Subscription>,
}
impl EventEmitter<Navigate> for ResourcesView {}

impl ResourcesView {
    /// Shows a package on the installed packages page.
    fn reveal_package(&mut self, source: String, cx: &mut Context<Self>) {
        self.expanded_packages.insert(format!("false:{source}"));
        self.browsing = false;
        cx.emit(Navigate::Package);
        cx.notify();
    }

    /// Row commands shared by the visible menu button and the context menu.
    fn resource_menu(
        &self,
        resource: &Resource,
        cx: &Context<Self>,
    ) -> impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static {
        let owner = cx.entity().downgrade();
        let busy = self.busy(cx);
        let opening = self.open.is_running();
        let resource = resource.clone();
        let row = self.row(&resource, cx);
        let files = self.owner_of(&resource, cx);
        let pi_config = self.pi_config.clone();
        let writable = resource.editable && row != Row::Inherited;
        move |menu, _, cx| {
            let mut menu = menu;
            if writable {
                let owner = owner.clone();
                let open = Open {
                    path: resource.path.clone(),
                    kind: resource.kind,
                    editable: true,
                    create: false,
                    owner: files.clone(),
                };
                menu = menu.item(
                    PopupMenuItem::new(t(cx, "settings-resource-edit"))
                        .icon(IconName::SquarePen)
                        .disabled(busy || opening)
                        .on_click(move |_, window, cx| {
                            owner
                                .update(cx, |this, cx| this.request_open(open.clone(), window, cx))
                                .ok();
                        }),
                );
            }
            let reveal = resource.path.clone();
            menu = menu.item(
                PopupMenuItem::new(t(cx, "settings-resource-location"))
                    .icon(IconName::FolderOpen)
                    .on_click(move |_, _, cx| cx.reveal_path(&reveal)),
            );
            if row != Row::Global {
                let copy = resource.path.display().to_string();
                menu = menu.item(
                    PopupMenuItem::new(t(cx, "files-copy-path"))
                        .icon(IconName::Copy)
                        .on_click(move |_, _, cx| {
                            cx.write_to_clipboard(ClipboardItem::new_string(copy.clone()))
                        }),
                );
            }
            if let Some(source) = resource.package.clone() {
                let owner = owner.clone();
                menu = menu.item(
                    PopupMenuItem::new(t(cx, "settings-resource-open-package"))
                        .icon(IconName::Package)
                        .on_click(move |_, _, cx| {
                            owner
                                .update(cx, |this, cx| this.reveal_package(source.clone(), cx))
                                .ok();
                        }),
                );
            }
            if row == Row::Inherited
                && let Some(pi_config) = pi_config.clone()
            {
                menu = menu.item(
                    PopupMenuItem::new(t(cx, "settings-resource-edit-global"))
                        .icon(IconName::SquarePen)
                        .on_click(move |_, window, cx| {
                            super::pi_config::request_scope(&pi_config, Scope::Global, window, cx)
                        }),
                );
            }
            if writable {
                let owner = owner.clone();
                let remove = resource.clone();
                menu = menu.separator().item(
                    PopupMenuItem::new(t(cx, "settings-resource-delete-ellipsis"))
                        .icon(IconName::Trash)
                        .disabled(busy)
                        .on_click(move |_, window, cx| {
                            owner
                                .update(cx, |this, cx| {
                                    this.confirm_remove(
                                        Some(remove.clone()),
                                        remove.name.clone(),
                                        window,
                                        cx,
                                    )
                                })
                                .ok();
                        }),
                );
            }
            menu
        }
    }

    /// The source label: a button to the owning package, otherwise a plain tag.
    fn render_source(&self, resource: &Resource, cx: &Context<Self>) -> AnyElement {
        let source = self.resource_source(resource, cx);
        let Some(package) = resource.package.clone() else {
            return Tag::secondary()
                .small()
                .outline()
                .child(source)
                .into_any_element();
        };
        let name = self
            .controller
            .read(cx)
            .catalog()
            .data()
            .and_then(|catalog| catalog.packages.iter().find(|p| p.source == package))
            .map(packages::package_name)
            .unwrap_or_else(|| package.strip_prefix("npm:").unwrap_or(&package).to_owned());
        let mut args = fluent_bundle::FluentArgs::new();
        args.set("package", name);
        Button::new(SharedString::from(format!(
            "from-package-{}",
            resource.path.display()
        )))
        .xsmall()
        .outline()
        .icon(IconName::Package)
        .label(gupi_settings::i18n::t_with_args(
            cx,
            "settings-resource-from-package",
            &args,
        ))
        .tooltip(source)
        .on_click(cx.listener(move |this, _, _, cx| this.reveal_package(package.clone(), cx)))
        .into_any_element()
    }

    fn resource_source(&self, resource: &Resource, cx: &App) -> String {
        if self.row(resource, cx) == Row::Own {
            return t(
                cx,
                if resource.is_project_owned() {
                    "settings-source-project"
                } else {
                    "settings-source-external"
                },
            );
        }
        resource.package.clone().unwrap_or_else(|| {
            let personal = resource.editable
                || self
                    .controller
                    .read(cx)
                    .catalog()
                    .data()
                    .is_some_and(|catalog| resource.path.starts_with(&catalog.root));
            t(
                cx,
                if personal {
                    "settings-source-personal"
                } else {
                    "settings-source-external"
                },
            )
        })
    }

    pub(super) fn load_preview(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let worker = cx.background_spawn({
            let path = path.clone();
            async move { std::fs::read_to_string(path).map_err(io::Error::from) }
        });
        let target = path.clone();
        let task = cx.spawn(async move |owner, cx| {
            let result = worker.await;
            let _ = owner.update(cx, |this, cx| {
                if let Some(preview) = this.previews.get_mut(&target) {
                    preview.transition(Complete(result));
                }
                cx.notify();
            });
        });
        let mut preview = refresh::Operation::new();
        preview.transition(Load(task));
        self.previews.insert(path, preview);
        cx.notify();
    }

    pub fn new(
        config: Entity<ConfigController>,
        applied_pi: Entity<PiProbeController>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let controller = cx.new(|_| ResourceController::new());
        let source = cx.new(|cx| InputState::new(window, cx));
        let names: [Entity<InputState>; 2] =
            std::array::from_fn(|_| cx.new(|cx| InputState::new(window, cx)));
        let search =
            cx.new(|cx| InputState::new(window, cx).placeholder(t(cx, "settings-skill-search")));
        let prompt_search =
            cx.new(|cx| InputState::new(window, cx).placeholder(t(cx, "settings-template-search")));
        let observe = cx.observe(&controller, |_, _, cx| cx.notify());
        let change = cx.subscribe(&search, |_, _, _: &InputEvent, cx| cx.notify());
        let saved = cx.subscribe_in(&controller, window, |this, origin, event, window, cx| {
            this.on_resource_event(origin, event, window, cx)
        });
        let (browse, browse_query) = browse::Browse::new(window, cx);
        let mut subscriptions = vec![observe, change, saved, browse_query];
        for input in [&source, &names[0], &names[1], &prompt_search] {
            subscriptions.push(cx.subscribe(input, |_, _, _: &InputEvent, cx| cx.notify()));
        }
        Self {
            controller,
            project: None,
            pi_config: None,
            after_save: None,
            projects: Default::default(),
            config,
            applied_pi,
            source,
            names,
            search,
            prompt_search,
            previews: Default::default(),
            editor: None,
            open: refresh::Operation::new(),
            creating: None,
            installing: false,
            expanded_packages: Default::default(),
            browsing: false,
            browse,
            installed_scroll: ScrollHandle::new(),
            packages_width: None,
            package_results: Default::default(),
            error: None,
            _subscriptions: subscriptions,
        }
    }
    /// Feedback for a finished change, from either the global or project files.
    fn on_resource_event(
        &mut self,
        origin: &Entity<ResourceController>,
        event: &ResourceEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            ResourceEvent::CatalogChanged => {
                let catalogs: Vec<io::Catalog> = std::iter::once(&self.controller)
                    .chain(self.project.as_ref())
                    .filter_map(|c| c.read(cx).catalog().data().cloned())
                    .collect();
                self.previews.retain(|path, _| {
                    catalogs
                        .iter()
                        .flat_map(|catalog| &catalog.resources)
                        .any(|r| matches!(r.kind, Kind::Skill | Kind::Prompt) && r.path == *path)
                });
                cx.notify();
            }
            ResourceEvent::Finished {
                target,
                result,
                change,
            } => {
                if let Change::Package { source, .. } = change
                    && origin == self.active()
                    && let Some(name) = gupi_resources::catalog::installed_name(source)
                {
                    self.package_results.insert(name.to_owned(), result.clone());
                }
                if let (Change::Save { path, .. }, Err(error)) = (change, result) {
                    if let Some(editor) = self.editor.as_mut()
                        && &editor.path == path
                    {
                        editor.changed = error.is_changed();
                    }
                    // A failed save keeps the editor open; leaving waits for the user.
                    if self.after_save.as_ref().is_some_and(|(p, _)| p == path) {
                        self.after_save = None;
                    }
                }
                let mut args = fluent_bundle::FluentArgs::new();
                let target = match origin.read(cx).target() {
                    Target::Project(cwd) => format!("{} — {target}", cwd.display()),
                    Target::Global => target.clone(),
                };
                args.set("target", target);
                if matches!(change, Change::Package { .. })
                    && origin == self.active()
                    && let Some(config) = &self.pi_config
                    && !config.read(cx).has_unsaved()
                {
                    config.update(cx, |config, cx| config.reload(cx));
                }
                let message = gupi_settings::i18n::t_with_args(
                    cx,
                    if result.is_ok() {
                        "settings-resource-success"
                    } else {
                        "settings-resource-failed"
                    },
                    &args,
                );
                window.push_notification(
                    match result {
                        Ok(()) => Notification::info(message),
                        Err(error) if error.is_changed() => Notification::error(format!(
                            "{message}\n{}",
                            t(cx, "settings-editor-changed")
                        )),
                        Err(error) => Notification::error(format!("{message}\n{error}")),
                    },
                    cx,
                );
            }
            ResourceEvent::Saved(path, text) => {
                if self.previews.contains_key(path) {
                    self.load_preview(path.clone(), cx);
                }
                if self.editor.as_ref().is_some_and(|editor| {
                    &editor.path == path && TextDraft::TEXT.get(&editor.form, cx) == *text
                }) {
                    self.editor = None;
                    window.close_dialog(cx);
                }
                if self.after_save.as_ref().is_some_and(|(p, _)| p == path)
                    && let Some((_, proceed)) = self.after_save.take()
                {
                    // Continue outside this update: leaving may read this view.
                    window.defer(cx, proceed);
                }
                cx.notify();
            }
        }
    }

    /// Follows the shared Pi scope: a project scope shows that project's files.
    pub(super) fn follow_scope(
        &mut self,
        pi_config: Entity<super::pi_config::PiConfig>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let observe = cx.observe_in(&pi_config, window, |this, _, window, cx| {
            this.sync_scope(window, cx)
        });
        self._subscriptions.push(observe);
        self.pi_config = Some(pi_config);
        self.sync_scope(window, cx);
    }

    fn sync_scope(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let target = match self.pi_config.as_ref().map(|c| c.read(cx).scope().clone()) {
            Some(Scope::Project(cwd)) => Target::Project(cwd),
            _ => Target::Global,
        };
        let current = self.project.as_ref().map(|c| c.read(cx).target().clone());
        if current.as_ref().unwrap_or(&Target::Global) == &target {
            return;
        }
        // Keep project controllers alive so an in-flight package command owns
        // its original cwd even when another scope is shown.
        self.creating = None;
        self.installing = false;
        self.source
            .update(cx, |source, cx| source.set_value("", window, cx));
        self.error = None;
        self.package_results.clear();
        self.expanded_packages.clear();
        self.project = None;
        if let Target::Project(cwd) = target {
            if !self.projects.contains_key(&cwd) {
                let controller =
                    cx.new(|_| ResourceController::for_target(Target::Project(cwd.clone())));
                let subscriptions = vec![
                    cx.observe(&controller, |_, _, cx| cx.notify()),
                    cx.subscribe_in(&controller, window, |this, origin, event, window, cx| {
                        this.on_resource_event(origin, event, window, cx)
                    }),
                ];
                self.projects.insert(
                    cwd.clone(),
                    ProjectResources {
                        controller,
                        _subscriptions: subscriptions,
                    },
                );
            }
            let controller = self.projects[&cwd].controller.clone();
            controller.update(cx, |c, cx| c.refresh(cx));
            self.project = Some(controller);
        }
        cx.notify();
    }

    pub(super) fn in_project_scope(&self) -> bool {
        self.project.is_some()
    }

    pub(super) fn stop(&self, cx: &mut Context<Self>) {
        self.controller.update(cx, |owner, _| owner.stop());
        for project in self.projects.values() {
            project.controller.update(cx, |owner, _| owner.stop());
        }
    }

    /// The controller listing resources in the current scope.
    fn active(&self) -> &Entity<ResourceController> {
        self.project.as_ref().unwrap_or(&self.controller)
    }

    fn busy(&self, cx: &App) -> bool {
        self.controller.read(cx).busy()
            || self.project.as_ref().is_some_and(|c| c.read(cx).busy())
            || self.config.read(cx).busy(cx)
    }

    fn in_project(&self, resource: &Resource, cx: &App) -> bool {
        self.project.as_ref().is_some_and(|project| {
            project
                .read(cx)
                .catalog()
                .data()
                .is_some_and(|catalog| catalog.resources.iter().any(|r| r == resource))
        })
    }

    fn row(&self, resource: &Resource, cx: &App) -> Row {
        match &self.project {
            None => Row::Global,
            Some(_) if self.in_project(resource, cx) => Row::Own,
            Some(_) => Row::Inherited,
        }
    }

    /// The controller whose files contain `resource`.
    fn owner_of(&self, resource: &Resource, cx: &App) -> Entity<ResourceController> {
        match &self.project {
            Some(project) if self.in_project(resource, cx) => project.clone(),
            _ => self.controller.clone(),
        }
    }

    /// The configured same-name relation between project and global entries.
    /// Shown only when Pi would load both: a trusted project, both enabled,
    /// and names Pi can determine. Never a claim about a running session.
    fn relation(&self, resource: &Resource, cx: &App) -> Option<String> {
        let row = self.row(resource, cx);
        if row == Row::Global {
            return None;
        }
        let trusted = self.pi_config.as_ref().and_then(|c| c.read(cx).trusted());
        if row == Row::Own && trusted == Some(false) {
            return Some(t(cx, "settings-resource-not-loaded"));
        }
        if trusted != Some(true) || !resource.enabled {
            return None;
        }
        let key = resource.key.as_ref()?;
        let other = if row == Row::Own {
            &self.controller
        } else {
            self.project.as_ref()?
        };
        let shared = other.read(cx).catalog().data().is_some_and(|catalog| {
            catalog
                .resources
                .iter()
                .any(|r| r.kind == resource.kind && r.enabled && r.key.as_ref() == Some(key))
        });
        shared.then(|| {
            t(
                cx,
                if row == Row::Own {
                    "settings-resource-same-global"
                } else {
                    "settings-resource-same-project"
                },
            )
        })
    }

    /// Whether the resource editor holds text that is not saved.
    pub(super) fn has_unsaved(&self, cx: &App) -> bool {
        self.editor
            .as_ref()
            .is_some_and(|editor| editor.editable && editor.form.read(cx).is_dirty())
    }

    /// Runs `proceed` once unsaved editor text is saved or discarded.
    pub(super) fn confirm_leave(
        this: &Entity<Self>,
        proceed: impl FnOnce(&mut Window, &mut App) + 'static,
        window: &mut Window,
        cx: &mut App,
    ) {
        if !this.read(cx).has_unsaved(cx) {
            proceed(window, cx);
            return;
        }
        let file = this
            .read(cx)
            .editor
            .as_ref()
            .and_then(|editor| editor.path.file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let this = this.clone();
        let proceed = std::rc::Rc::new(std::cell::RefCell::new(Some(Box::new(proceed) as Proceed)));
        window.open_dialog(cx, move |dialog, _, cx| {
            let discard = (this.clone(), proceed.clone());
            let save = (this.clone(), proceed.clone());
            let mut args = fluent_bundle::FluentArgs::new();
            args.set("file", file.clone());
            dialog
                .title(gupi_settings::i18n::t_with_args(
                    cx,
                    "settings-editor-unsaved-title",
                    &args,
                ))
                .child(t(cx, "settings-editor-unsaved-body"))
                .footer(
                    gpui_kit::component::dialog::DialogFooter::new()
                        .child(
                            Button::new("resource-unsaved-cancel")
                                .label(t(cx, "action-cancel"))
                                .on_click(|_, window, cx| window.close_dialog(cx)),
                        )
                        .child(
                            Button::new("resource-unsaved-discard")
                                .label(t(cx, "pi-settings-discard"))
                                .on_click(move |_, window, cx| {
                                    // Close this question, then the editor beneath it.
                                    window.close_dialog(cx);
                                    discard.0.update(cx, |this, cx| {
                                        this.editor = None;
                                        cx.notify();
                                    });
                                    window.close_dialog(cx);
                                    if let Some(proceed) = discard.1.borrow_mut().take() {
                                        proceed(window, cx);
                                    }
                                }),
                        )
                        .child(
                            Button::new("resource-unsaved-save")
                                .primary()
                                .label(t(cx, "settings-resource-save"))
                                .on_click(move |_, window, cx| {
                                    window.close_dialog(cx);
                                    let Some(proceed) = save.1.borrow_mut().take() else {
                                        return;
                                    };
                                    save.0.update(cx, |this, cx| this.save_then(proceed, cx));
                                }),
                        ),
                )
        });
    }

    fn change(&mut self, change: Change, cx: &mut Context<Self>) {
        let controller = self.controller.clone();
        self.change_in(&controller, change, cx);
    }

    fn change_in(
        &mut self,
        controller: &Entity<ResourceController>,
        change: Change,
        cx: &mut Context<Self>,
    ) {
        if self.config.read(cx).busy(cx) {
            return;
        }
        self.error = None;
        controller.update(cx, |c, cx| c.change(change, cx));
    }
    fn package_available(&self, cx: &App) -> bool {
        let controller = self.active().read(cx);
        let config = self.config.read(cx);
        (!self.in_project_scope()
            || self.pi_config.as_ref().and_then(|c| c.read(cx).trusted()) == Some(true))
            && !controller.busy()
            && controller.catalog().data().is_some()
            && controller.catalog().problem().is_none()
            && !config.busy(cx)
            && self
                .applied_pi
                .read(cx)
                .ready_for(config.preferences(cx).pi_command.as_deref())
                .is_some()
    }

    fn package(&mut self, action: &'static str, source: String, cx: &mut Context<Self>) {
        let command = self.config.read(cx).preferences(cx).pi_command;
        let Some(command) = self
            .applied_pi
            .read(cx)
            .ready_for(command.as_deref())
            .map(|p| p.command.clone())
        else {
            self.error = Some((Kind::Extension, t(cx, "settings-resource-pi-required")));
            cx.notify();
            return;
        };
        if !self.package_available(cx) {
            return;
        }
        if let Some(name) = gupi_resources::catalog::installed_name(&source) {
            self.package_results.remove(name);
        }
        if let Some(cwd) = self
            .project
            .as_ref()
            .and_then(|p| match p.read(cx).target() {
                Target::Project(cwd) => Some(cwd.clone()),
                _ => None,
            })
            && let Err(error) = io::project_package_allowed(&cwd, action, &source)
        {
            self.error = Some((Kind::Extension, error.to_string()));
            cx.notify();
            return;
        }
        let controller = self.active().clone();
        self.change_in(
            &controller,
            Change::Package {
                command,
                action,
                source,
            },
            cx,
        );
    }
    fn name_input(&self, kind: Kind) -> &Entity<InputState> {
        &self.names[usize::from(kind == Kind::Prompt)]
    }
    fn new_resource(&mut self, kind: Kind, window: &mut Window, cx: &mut Context<Self>) {
        let owner = self.active().clone();
        let Some(root) = owner.read(cx).catalog().data().map(|c| c.root.clone()) else {
            return;
        };
        match io::create_path(&root, kind, &self.name_input(kind).read(cx).value()) {
            Ok(path) => self.request_open(
                Open {
                    path,
                    kind,
                    editable: true,
                    create: true,
                    owner,
                },
                window,
                cx,
            ),
            Err(_) => {
                self.error = Some((kind, t(cx, "settings-resource-invalid-name")));
                cx.notify();
            }
        }
    }
    fn pick_skill(&mut self, cx: &mut Context<Self>) {
        let prompt = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: true,
            multiple: false,
            prompt: Some(t(cx, "settings-resource-register").into()),
        });
        cx.spawn(async move |owner, cx| {
            if let Ok(Ok(Some(paths))) = prompt.await
                && let Some(path) = paths.into_iter().next()
            {
                let _ = owner.update(cx, |this, cx| this.change(Change::RegisterSkill(path), cx));
            }
        })
        .detach();
    }
    fn confirm_remove(
        &mut self,
        resource: Option<Resource>,
        source: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let owner = cx.entity().downgrade();
        let target = self.active().clone();
        window.open_dialog(cx, move |dialog, _, cx| {
            let owner = owner.clone();
            let target = target.clone();
            let resource = resource.clone();
            let source = source.clone();
            dialog
                .footer(super::dialog_buttons(
                    if resource.is_some() {
                        "settings-resource-delete"
                    } else {
                        "settings-package-remove"
                    },
                    false,
                    false,
                    cx,
                ))
                .title(t(
                    cx,
                    if resource.is_some() {
                        "settings-resource-delete"
                    } else {
                        "settings-package-remove"
                    },
                ))
                .child(div().font_weight(FontWeight::MEDIUM).child(source.clone()))
                // The exact file moved to the Trash; a skill's folder stays.
                .children(resource.as_ref().map(|resource| {
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(resource.path.display().to_string())
                }))
                .child(t(
                    cx,
                    if resource.is_some() {
                        "settings-resource-confirm-remove"
                    } else {
                        "settings-package-confirm-remove"
                    },
                ))
                .on_ok(move |_, _, cx| {
                    owner
                        .update(cx, |this, cx| {
                            if this.busy(cx) || &target != this.active() {
                                return false;
                            }
                            if let Some(resource) = &resource {
                                let files = this.owner_of(resource, cx);
                                this.change_in(&files, Change::Delete(resource.clone()), cx);
                            } else {
                                this.package("remove", source.clone(), cx);
                            }
                            true
                        })
                        .unwrap_or(false)
                })
        });
    }
    pub fn render_header(&self, kind: Kind, cx: &Context<Self>) -> AnyElement {
        let controller = self.controller.read(cx);
        h_flex()
            .gap_1()
            .when(kind == Kind::Extension, |header| {
                header.child(div().mr_2().child(self.render_packages_tabs(cx)))
            })
            .child(
                Button::new("resources-help")
                    .ghost()
                    .small()
                    .icon(IconName::Info)
                    .tooltip(t(cx, "settings-resource-reload-help")),
            )
            .child(
                Button::new("resources-refresh")
                    .ghost()
                    .small()
                    .icon(IconName::RotateCw)
                    .tooltip(t(cx, "settings-resource-refresh"))
                    .loading(
                        controller.catalog().is_running()
                            || self
                                .project
                                .as_ref()
                                .is_some_and(|c| c.read(cx).catalog().is_running()),
                    )
                    .disabled(self.busy(cx))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.controller.update(cx, |c, cx| c.refresh(cx));
                        if let Some(project) = &this.project {
                            project.update(cx, |c, cx| c.refresh(cx));
                        }
                        for path in this.previews.keys().cloned().collect::<Vec<_>>() {
                            this.load_preview(path, cx);
                        }
                    })),
            )
            .into_any_element()
    }
    /// All resource pages share the selected Pi scope.
    fn page_controller(&self, _kind: Kind) -> &Entity<ResourceController> {
        self.active()
    }

    pub fn has_status(&self, kind: Kind, cx: &App) -> bool {
        let controller = self.page_controller(kind).read(cx);
        controller.mutation().is_running()
            || controller.catalog().problem().is_some()
            || self.open.problem().is_some()
            || self.open.is_running()
            || self
                .error
                .as_ref()
                .is_some_and(|(error_kind, _)| *error_kind == kind)
            || controller
                .catalog()
                .data()
                .is_some_and(|catalog| !catalog.warnings.is_empty())
    }
    pub fn render_page(
        &mut self,
        kind: Kind,
        section: Section,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if matches!(
            self.controller.read(cx).catalog(),
            refresh::Operation::Idle(_)
        ) {
            let controller = self.controller.clone();
            cx.defer(move |cx| {
                controller.update(cx, |owner, cx| {
                    if matches!(owner.catalog(), refresh::Operation::Idle(_)) {
                        owner.refresh(cx);
                    }
                })
            });
        }
        if kind == Kind::Extension {
            let probe = self.applied_pi.clone();
            let command = self.config.read(cx).preferences(cx).pi_command;
            if !probe.read(cx).matches_command(command.as_deref()) {
                cx.defer(move |cx| probe.update(cx, |probe, cx| probe.request(command, false, cx)));
            }
        }
        if kind != Kind::Extension
            && let Some(project) = &self.project
            && matches!(project.read(cx).catalog(), refresh::Operation::Idle(_))
        {
            let project = project.clone();
            cx.defer(move |cx| project.update(cx, |c, cx| c.refresh(cx)));
        }
        let busy = self.busy(cx);
        let controller = self.page_controller(kind).read(cx);
        let catalog = controller.catalog().data();
        let mut view = v_flex().gap_3();
        if section == Section::Overview {
            if controller.mutation().is_running() {
                view = view.child(t(cx, "settings-resource-working"));
            }
            for error in controller
                .catalog()
                .problem()
                .into_iter()
                .chain(self.open.problem())
            {
                view = view.child(div().text_color(cx.theme().danger).child(error.to_string()));
            }
            if let Some((error_kind, error)) = &self.error
                && *error_kind == kind
            {
                view = view.child(div().text_color(cx.theme().danger).child(error.clone()));
            }
            if self.open.is_running() {
                view = view.child(t(cx, "settings-resource-loading"));
            }
            if let Some(catalog) = catalog {
                for warning in &catalog.warnings {
                    view = view.child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(warning.clone()),
                    );
                }
            }
            return view.into_any_element();
        }
        if section == Section::Packages {
            // Settings 0.7.1 mounts pages in its group list, which measures each
            // item at its content height, so the body must keep its natural
            // height for the page list to scroll. A container query cannot be
            // used here: its contents never contribute to its size. Layout
            // decisions follow the body's own width, measured after layout.
            // Until the first layout reports a width, assume a typical page.
            let width = self.packages_width.unwrap_or(px(640.));
            let owner = cx.entity().downgrade();
            let measured = self.packages_width;
            return div()
                .relative()
                .w_full()
                .min_w_0()
                .child(self.render_packages(width, packages::Mount::Content, cx))
                .on_prepaint(move |bounds, _, cx| {
                    let width = bounds.size.width;
                    if measured.is_none_or(|last| (last - width).abs() >= px(0.5)) {
                        let _ = owner.update(cx, |this, cx| {
                            this.packages_width = Some(width);
                            cx.notify();
                        });
                    }
                })
                .into_any_element();
        }
        if catalog.is_none() {
            return view.into_any_element();
        }
        if kind != Kind::Extension {
            // Library toolbar: search, then the ordinary create command.
            let search = if kind == Kind::Prompt {
                &self.prompt_search
            } else {
                &self.search
            };
            let mut toolbar =
                h_flex()
                    .flex_wrap()
                    .gap_2()
                    .child(div().flex_1().min_w(px(160.)).child(
                        Input::new(search).prefix(gpui_kit::component::Icon::new(IconName::Search)),
                    ));
            toolbar = toolbar.child(
                Button::new("resource-add")
                    .icon(IconName::Plus)
                    .label(t(cx, "settings-resource-create"))
                    .disabled(busy)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.creating = Some(kind);
                        this.name_input(kind).focus_handle(cx).focus(window, cx);
                        cx.notify();
                    })),
            );
            if kind == Kind::Skill && self.project.is_none() {
                toolbar = toolbar.child(
                    Button::new("resource-register")
                        .ghost()
                        .label(t(cx, "settings-resource-register"))
                        .disabled(busy)
                        .on_click(cx.listener(|this, _, _, cx| this.pick_skill(cx))),
                );
            }
            view = view.child(toolbar);
            if self.creating == Some(kind) {
                view = view
                    .child(
                        gpui_kit::component::form::field()
                            .label(t(cx, "settings-resource-name"))
                            .description(t(cx, "settings-resource-name-help"))
                            .child(Input::new(self.name_input(kind)).disabled(busy)),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Button::new("resource-create")
                                    .primary()
                                    .label(t(cx, "settings-resource-create"))
                                    .disabled(
                                        busy || self
                                            .name_input(kind)
                                            .read(cx)
                                            .value()
                                            .trim()
                                            .is_empty(),
                                    )
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.new_resource(kind, window, cx)
                                    })),
                            )
                            .child(
                                Button::new("resource-create-cancel")
                                    .ghost()
                                    .label(t(cx, "action-cancel"))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.creating = None;
                                        cx.notify();
                                    })),
                            ),
                    );
            }
        }
        if kind == Kind::Skill {
            if self.filtered_skills(cx).is_empty() {
                view = view.child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(t(
                            cx,
                            if !self.search.read(cx).value().trim().is_empty() {
                                "settings-resource-empty"
                            } else if self.project.is_some() {
                                "settings-resource-none-project"
                            } else {
                                "settings-resource-none"
                            },
                        )),
                );
            }
            return view.into_any_element();
        }
        if kind == Kind::Prompt && self.filtered_prompts(cx).is_empty() {
            view = view.child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(t(
                        cx,
                        if !self.prompt_search.read(cx).value().trim().is_empty() {
                            "settings-resource-empty"
                        } else if self.project.is_some() {
                            "settings-resource-none-project"
                        } else {
                            "settings-resource-none"
                        },
                    )),
            );
        }
        view.into_any_element()
    }
}
fn io_counts(catalog: &io::Catalog, source: &str, cx: &App) -> Option<Vec<String>> {
    let mut counts = Vec::new();
    for (kind, label) in [
        (Kind::Extension, "settings-package-extensions"),
        (Kind::Skill, "settings-package-skills"),
        (Kind::Theme, "settings-package-themes"),
        (Kind::Prompt, "settings-package-prompts"),
    ] {
        let count = catalog
            .resources
            .iter()
            .filter(|r| r.package.as_deref() == Some(source) && r.kind == kind)
            .count();
        if count > 0 {
            let mut args = fluent_bundle::FluentArgs::new();
            args.set("count", count as i64);
            counts.push(gupi_settings::i18n::t_with_args(cx, label, &args));
        }
    }
    (!counts.is_empty()).then_some(counts)
}

// The name and description already appear in the card; preview only the Markdown body.
fn preview_body(text: &str) -> &str {
    let mut lines = text.split_inclusive('\n');
    if lines.next().is_some_and(|line| line.trim() == "---") {
        let mut offset = text.find('\n').map_or(text.len(), |index| index + 1);
        for line in lines {
            offset += line.len();
            if line.trim() == "---" {
                return text[offset..].trim_start();
            }
        }
    }
    text
}
