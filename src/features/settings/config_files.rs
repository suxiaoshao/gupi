//! Independent, scope-local drafts for models.json and mcp.json.
use crate::features::settings::composition::{SettingsPage, SettingsRow, SettingsSection};
mod editor;
mod mcp;
use editor::EntryEditor;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::group_box::GroupBoxVariants as _;
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Sizable as _, WindowExt as _, h_flex, v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use gupi_resources::pi_config_files::{Entry, Error, File, Kind};
use gupi_resources::pi_settings::{Scope, ordered_eq};
use gupi_settings::{
    assets::IconName,
    i18n::{t, t_with_args},
};
use serde_json::Value;
use std::{collections::BTreeSet, path::PathBuf};

type Continue = Box<dyn FnOnce(&mut Window, &mut App)>;
#[derive(Default)]
struct Draft {
    file: Option<File>,
    edits: Vec<(String, Option<Entry>)>,
    auto: Option<Option<Value>>,
    error: Option<Error>,
}
impl Draft {
    fn dirty(&self) -> bool {
        !self.edits.is_empty() || self.auto.is_some()
    }
    fn entry(&self, name: &str) -> Option<Entry> {
        self.edits
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, e)| e.clone())
            .unwrap_or_else(|| self.file.as_ref().and_then(|f| f.entry(name)))
    }
    fn names(&self) -> Result<Vec<String>, Error> {
        let mut names = self.file.as_ref().ok_or(Error::Read)?.names()?;
        for (name, entry) in &self.edits {
            if entry.is_none() {
                names.retain(|n| n != name)
            } else if !names.contains(name) {
                names.push(name.clone())
            }
        }
        Ok(names)
    }
    fn edit(&mut self, name: String, entry: Option<Entry>) {
        let baseline = self.file.as_ref().and_then(|f| f.entry(&name));
        let unchanged = match (&baseline, &entry) {
            (None, None) => true,
            (Some(a), Some(b)) => a.same(b),
            _ => false,
        };
        if let Some(ix) = self.edits.iter().position(|(n, _)| n == &name) {
            if unchanged {
                self.edits.remove(ix);
            } else {
                self.edits[ix].1 = entry;
            }
        } else if !unchanged {
            self.edits.push((name, entry));
        }
    }
}
impl EventEmitter<super::CredentialPage> for FilePages {}

pub(super) struct FilePages {
    agent: Result<PathBuf, Error>,
    scope: Scope,
    models: Draft,
    mcp: Draft,
    global_mcp: Option<Result<File, Error>>,
    generation: u64,
    loading: Option<Task<()>>,
    saving: Option<Task<()>>,
    continuation: Option<Continue>,
    auth_providers: Option<BTreeSet<String>>,
    auth_loading: Option<Task<()>>,
    auth_generation: u64,
    mcp_actions: mcp::Actions,
    enabled_inputs: std::collections::BTreeMap<String, EnabledInput>,
}
#[derive(Clone, PartialEq)]
struct EnabledChoice {
    value: Option<bool>,
    title: SharedString,
}
impl gpui_kit::component::searchable_list::SearchableListItem for EnabledChoice {
    type Value = Option<bool>;
    fn title(&self) -> SharedString {
        self.title.clone()
    }
    fn value(&self) -> &Self::Value {
        &self.value
    }
}
struct EnabledInput {
    state: Entity<
        gpui_kit::component::select::SelectState<
            gpui_kit::component::searchable_list::SearchableVec<EnabledChoice>,
        >,
    >,
    choices: Vec<EnabledChoice>,
    _subscription: Subscription,
}
impl FilePages {
    fn render_enabled(
        &mut self,
        name: Option<&str>,
        value: Option<Value>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        use gpui_kit::component::searchable_list::SearchableVec;
        use gpui_kit::component::select::{Select, SelectEvent, SelectState};
        let key = name
            .map(|name| format!("server:{name}"))
            .unwrap_or_else(|| "auto".into());
        let choices = [
            (None, "pi-value-unset"),
            (Some(true), "pi-files-on"),
            (Some(false), "pi-files-off"),
        ]
        .into_iter()
        .map(|(value, key)| EnabledChoice {
            value,
            title: t(cx, key).into(),
        })
        .collect::<Vec<_>>();
        if !self.enabled_inputs.contains_key(&key) {
            let state = cx
                .new(|cx| SelectState::new(SearchableVec::new(choices.clone()), None, window, cx));
            let name = name.map(str::to_owned);
            let generation = self.generation;
            let subscription = cx.subscribe(&state, move |this, _, event, cx| {
                let SelectEvent::<SearchableVec<EnabledChoice>>::Confirm(Some(value)) = event
                else {
                    return;
                };
                if this.generation != generation || this.busy() {
                    return;
                }
                let value = value.map(Value::Bool);
                if let Some(name) = &name {
                    if let Some(mut entry) = this.mcp.entry(name) {
                        if let Err(error) = entry.set(&["enabled".into()], value) {
                            this.mcp.error = Some(error);
                        } else {
                            this.mcp.edit(name.clone(), Some(entry));
                        }
                        cx.notify();
                    }
                } else {
                    this.set_auto(value, cx);
                }
            });
            self.enabled_inputs.insert(
                key.clone(),
                EnabledInput {
                    state,
                    choices: choices.clone(),
                    _subscription: subscription,
                },
            );
        }
        let input = self
            .enabled_inputs
            .get_mut(&key)
            .expect("retained enabled input");
        if input.choices != choices {
            input.choices = choices.clone();
            input.state.update(cx, |state, cx| {
                state.set_items(SearchableVec::new(choices), window, cx)
            });
        }
        let current = match value {
            None => Some(None),
            Some(Value::Bool(value)) => Some(Some(value)),
            _ => None,
        };
        if input.state.read(cx).selected_value() != current.as_ref() {
            input.state.update(cx, |state, cx| {
                if let Some(value) = &current {
                    state.set_selected_value(value, window, cx);
                } else {
                    state.set_selected_index(None, window, cx);
                }
            });
        }
        let state = input.state.clone();
        let label = name
            .map(|name| format!("{name}: {}", t(cx, "pi-files-label-enabled")))
            .unwrap_or_else(|| t(cx, "pi-files-label-autoEnableCodemode"));
        div()
            .w(rems(7.5))
            .flex_none()
            .child(
                Select::new(&state)
                    .small()
                    .w_full()
                    .accessibility_label(label)
                    .placeholder(t(cx, "pi-resource-unsupported"))
                    .disabled(self.busy() || self.loading.is_some() || self.mcp.file.is_none()),
            )
            .into_any_element()
    }
}

impl FilePages {
    pub(super) fn new(_: &mut Context<Self>) -> Self {
        Self {
            agent: gupi_resources::pi_resources::agent_dir().map_err(|_| Error::Read),
            scope: Scope::Global,
            models: Draft::default(),
            mcp: Draft::default(),
            global_mcp: None,
            generation: 0,
            loading: None,
            saving: None,
            continuation: None,
            auth_providers: None,
            auth_loading: None,
            auth_generation: 0,
            mcp_actions: mcp::Actions::default(),
            enabled_inputs: Default::default(),
        }
    }
    #[cfg(test)]
    pub(super) fn set_agent_for_test(&mut self, agent: PathBuf) {
        self.agent = Ok(agent);
        self.auth_generation += 1;
        self.auth_loading = None;
        self.auth_providers = None;
    }
    fn draft(&self, kind: Kind) -> &Draft {
        match kind {
            Kind::Models => &self.models,
            Kind::Mcp => &self.mcp,
        }
    }
    fn draft_mut(&mut self, kind: Kind) -> &mut Draft {
        match kind {
            Kind::Models => &mut self.models,
            Kind::Mcp => &mut self.mcp,
        }
    }
    pub(super) fn dirty(&self) -> bool {
        self.models.dirty() || self.mcp.dirty()
    }
    pub(super) fn busy(&self) -> bool {
        self.saving.is_some() || self.mcp_actions.running.is_some()
    }
    pub(super) fn set_scope(&mut self, scope: Scope, cx: &mut Context<Self>) {
        self.mcp_actions.invalidate();
        self.enabled_inputs.clear();
        self.scope = scope;
        self.models = Draft::default();
        self.mcp = Draft::default();
        self.loading = None;
        self.generation += 1;
        let _ = cx;
    }
    pub(super) fn activate(&mut self, cx: &mut Context<Self>) {
        if (self.models.file.is_some() || self.mcp.file.is_some()) && !self.dirty() && !self.busy()
        {
            self.reload(cx)
        }
    }
    fn refresh_auth(&mut self, cx: &mut Context<Self>) {
        // Entry and initial file load can request the same snapshot in one frame.
        if self.auth_loading.is_some() {
            return;
        }
        let Ok(agent) = self.agent.clone() else {
            return;
        };
        self.auth_generation += 1;
        let generation = self.auth_generation;
        let worker = cx.background_spawn(async move {
            gupi_resources::pi_auth::File::load(&agent.join("auth.json"))
                .ok()
                .map(|file| {
                    file.entries()
                        .iter()
                        .map(|entry| entry.provider().to_owned())
                        .collect()
                })
        });
        self.auth_loading = Some(cx.spawn(async move |this, cx| {
            let providers = worker.await;
            let _ = this.update(cx, |this, cx| {
                if this.auth_generation != generation {
                    return;
                }
                this.auth_loading = None;
                this.auth_providers = providers;
                cx.notify();
            });
        }));
    }
    fn has_stored_credentials(&self, name: &str) -> bool {
        self.auth_providers
            .as_ref()
            .is_some_and(|providers| providers.contains(name))
    }
    fn reload(&mut self, cx: &mut Context<Self>) {
        let Ok(agent) = self.agent.clone() else {
            return;
        };
        if self.busy() {
            return;
        }
        self.mcp_actions.invalidate();
        self.enabled_inputs.clear();
        self.refresh_auth(cx);
        self.generation += 1;
        let generation = self.generation;
        let scope = self.scope.clone();
        let read = cx.background_spawn(async move {
            let models = File::load(Kind::Models, &Kind::Models.file(&agent, &Scope::Global));
            let mcp = File::load(Kind::Mcp, &Kind::Mcp.file(&agent, &scope));
            let global = if matches!(scope, Scope::Project(_)) {
                Some(File::load(
                    Kind::Mcp,
                    &Kind::Mcp.file(&agent, &Scope::Global),
                ))
            } else {
                None
            };
            (models, mcp, global)
        });
        self.loading = Some(cx.spawn(async move |this, cx| {
            let (models, mcp, global) = read.await;
            let _ = this.update(cx, |this, cx| {
                if this.generation != generation {
                    return;
                }
                this.loading = None;
                for (kind, result) in [(Kind::Models, models), (Kind::Mcp, mcp)] {
                    let draft = this.draft_mut(kind);
                    match result {
                        Ok(file) => {
                            draft.file = Some(file);
                            draft.error = None
                        }
                        Err(e) => {
                            draft.file = None;
                            draft.error = Some(e)
                        }
                    }
                }
                this.global_mcp = global;
                cx.notify();
            });
        }));
        cx.notify();
    }
    fn save(
        &mut self,
        kind: Option<Kind>,
        then: Option<Continue>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.busy() {
            return;
        }
        let Ok(agent) = self.agent.clone() else {
            return;
        };
        self.mcp_actions.invalidate();
        self.enabled_inputs.clear();
        let scope = self.scope.clone();
        let mut requests = vec![];
        for k in [Kind::Models, Kind::Mcp] {
            let draft = self.draft(k);
            if kind.is_none_or(|kind| kind == k)
                && draft.dirty()
                && let Some(file) = &draft.file
            {
                requests.push((k, file.clone(), draft.edits.clone(), draft.auto.clone()));
            }
        }
        let task = cx.background_spawn(async move {
            requests
                .into_iter()
                .map(|(k, file, edits, auto)| {
                    let scope = if k == Kind::Models {
                        Scope::Global
                    } else {
                        scope.clone()
                    };
                    (k, file.commit(&agent, &scope, &edits, auto))
                })
                .collect::<Vec<_>>()
        });
        self.saving = Some(cx.spawn_in(window, async move |this, cx| {
            let results = task.await;
            let next = this.update(cx, |this, cx| {
                this.saving = None;
                let mut good = true;
                for (kind, result) in results {
                    let draft = this.draft_mut(kind);
                    match result {
                        Ok(file) => {
                            draft.file = Some(file);
                            draft.edits.clear();
                            draft.auto = None;
                            draft.error = None
                        }
                        Err(e) => {
                            draft.error = Some(e);
                            good = false
                        }
                    }
                }
                cx.notify();
                if good {
                    this.continuation.take().or(then)
                } else {
                    this.continuation = None;
                    None
                }
            });
            if let Ok(Some(next)) = next {
                let _ = cx.update(|window, cx| next(window, cx));
            }
        }));
        cx.notify();
    }
    pub(super) fn confirm_leave(
        entity: &Entity<Self>,
        proceed: Continue,
        window: &mut Window,
        cx: &mut App,
    ) {
        if entity.read(cx).busy() {
            entity.update(cx, |this, _| {
                this.mcp_actions.cancel();
                this.continuation = Some(proceed);
            });
            return;
        }
        if !entity.read(cx).dirty() {
            proceed(window, cx);
            return;
        }
        let entity = entity.clone();
        let proceed = std::rc::Rc::new(std::cell::RefCell::new(Some(proceed)));
        window.open_dialog(cx, move |dialog, _, cx| {
            let discard = (entity.clone(), proceed.clone());
            let save = (entity.clone(), proceed.clone());
            dialog
                .title(t(cx, "pi-settings-unsaved-title"))
                .child(t(cx, "pi-settings-unsaved-body"))
                .footer(
                    gpui_kit::component::dialog::DialogFooter::new()
                        .child(
                            Button::new("file-cancel")
                                .label(t(cx, "action-cancel"))
                                .on_click(|_, w, cx| w.close_dialog(cx)),
                        )
                        .child(
                            Button::new("file-discard")
                                .label(t(cx, "pi-settings-discard"))
                                .on_click(move |_, w, cx| {
                                    w.close_dialog(cx);
                                    discard.0.update(cx, |this, cx| {
                                        this.models.edits.clear();
                                        this.models.auto = None;
                                        this.mcp.edits.clear();
                                        this.mcp.auto = None;
                                        cx.notify()
                                    });
                                    if let Some(next) = discard.1.borrow_mut().take() {
                                        next(w, cx)
                                    }
                                }),
                        )
                        .child(
                            Button::new("file-save")
                                .primary()
                                .label(t(cx, "pi-settings-save"))
                                .on_click(move |_, w, cx| {
                                    w.close_dialog(cx);
                                    let next = save.1.borrow_mut().take();
                                    save.0.update(cx, |this, cx| this.save(None, next, w, cx));
                                }),
                        ),
                )
        });
    }
    pub(super) fn page(
        entity: &Entity<Self>,
        kind: Kind,
        config: &Entity<super::pi_config::PiConfig>,
        cx: &App,
    ) -> SettingsPage {
        let e = entity.clone();
        let h = entity.clone();
        let actions = entity.clone();
        let config = config.clone();
        let mcp_config = config.clone();
        let page = SettingsPage::new(t(
            cx,
            if kind == Kind::Models {
                "pi-files-models"
            } else {
                "pi-files-mcp"
            },
        ))
        .icon(if kind == Kind::Models {
            IconName::Boxes
        } else {
            IconName::Server
        })
        .scope(move |_, window, cx| {
            config.update(cx, |this, cx| {
                this.render_scope(kind == Kind::Models, window, cx)
            })
        })
        .actions(move |_, _, cx| actions.update(cx, |this, cx| this.render_file_actions(kind, cx)));
        let page =
            if entity.read(cx).draft(kind).dirty() {
                page.header(move |_, _, cx| {
                    let e = h.clone();
                    let dirty = h.read(cx).draft(kind).dirty();
                    let busy = h.read(cx).busy();
                    v_flex()
                        .w_full()
                        .min_w_0()
                        .gap_2()
                        .child(h_flex().w_full().justify_end().gap_2().flex_wrap().when(
                            dirty,
                            |row| {
                                row.child(
                                    div()
                                        .text_xs()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(t(cx, "settings-key-unsaved")),
                                )
                                .child(
                                    Button::new("file-reset")
                                        .small()
                                        .label(t(cx, "pi-settings-discard"))
                                        .disabled(busy)
                                        .on_click({
                                            let e = e.clone();
                                            move |_, _, cx| {
                                                e.update(cx, |this, cx| {
                                                    let draft = this.draft_mut(kind);
                                                    draft.edits.clear();
                                                    draft.auto = None;
                                                    cx.notify()
                                                })
                                            }
                                        }),
                                )
                                .child(
                                    Button::new("file-save-page")
                                        .small()
                                        .primary()
                                        .label(t(cx, "pi-settings-save"))
                                        .disabled(busy)
                                        .on_click({
                                            let e = e.clone();
                                            move |_, w, cx| {
                                                e.update(cx, |this, cx| {
                                                    this.save(Some(kind), None, w, cx)
                                                })
                                            }
                                        }),
                                )
                            },
                        ))
                        .into_any_element()
                })
            } else {
                page
            };
        let metadata = entity.clone();
        let page = page.group(SettingsSection::new().item(SettingsRow::content(
            move |layout, window, cx| {
                if kind == Kind::Models {
                    let owner = metadata.downgrade();
                    window.use_keyed_state("models-page-auth-metadata-entry", cx, |window, cx| {
                        window.defer(cx, move |_, cx| {
                            let _ = owner.update(cx, |this, cx| this.refresh_auth(cx));
                        });
                    });
                }
                let context = (
                    mcp_config.read(cx).mcp_command(cx),
                    mcp_config.read(cx).trusted() == Some(true),
                );
                e.update(cx, |this, cx| {
                    this.render_page_with_mcp(kind, context, layout.width, window, cx)
                })
            },
        )));
        if kind == Kind::Mcp {
            let e = entity.clone();
            page.group(
                SettingsSection::new()
                    .title(t(cx, "pi-files-automation"))
                    .item(
                        SettingsRow::content(move |layout, window, cx| {
                            e.update(cx, |this, cx| this.render_auto(layout, window, cx))
                        })
                        .framed(true),
                    ),
            )
        } else {
            page
        }
    }

    fn render_file_actions(&self, kind: Kind, cx: &mut Context<Self>) -> AnyElement {
        let path = self.agent.as_ref().ok().map(|agent| {
            kind.file(
                agent,
                if kind == Kind::Models {
                    &Scope::Global
                } else {
                    &self.scope
                },
            )
        });
        let mut args = fluent_bundle::FluentArgs::new();
        args.set(
            "filename",
            if kind == Kind::Models {
                "models.json"
            } else {
                "mcp.json"
            },
        );
        let open_label = t_with_args(cx, "pi-files-open-file", &args);
        h_flex()
            .gap_1()
            .child(
                Button::new("file-open")
                    .small()
                    .ghost()
                    .icon(IconName::FolderOpen)
                    .tooltip(open_label.clone())
                    .accessibility_label(open_label)
                    .disabled(self.busy() || path.as_ref().is_none_or(|p| !p.exists()))
                    .on_click(move |_, _, cx| {
                        if let Some(path) = &path {
                            cx.open_with_system(path);
                        }
                    }),
            )
            .child(
                Button::new("file-reload")
                    .small()
                    .ghost()
                    .icon(IconName::RotateCw)
                    .tooltip(t(cx, "pi-settings-reload"))
                    .accessibility_label(t(cx, "pi-settings-reload"))
                    .disabled(self.busy() || self.draft(kind).dirty())
                    .on_click(cx.listener(|this, _, _, cx| this.reload(cx))),
            )
            .into_any_element()
    }

    #[cfg(test)]
    fn render_page(
        &mut self,
        kind: Kind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.render_page_with_mcp(
            kind,
            (None, false),
            window.viewport_size().width,
            window,
            cx,
        )
    }
    fn render_page_with_mcp(
        &mut self,
        kind: Kind,
        context: (Option<std::ffi::OsString>, bool),
        width: Pixels,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let narrow = width < window.rem_size() * 41.25;
        let action_width = if narrow { rems(12.) } else { rems(15.) };
        let mount = (kind == Kind::Mcp).then(|| {
            window
                .use_keyed_state("mcp-cli-page-mount", cx, |_, _| mcp::Mount::default())
                .read(cx)
                .0
                .clone()
        });
        if self.draft(kind).file.is_none()
            && self.draft(kind).error.is_none()
            && self.loading.is_none()
        {
            let entity = cx.entity().downgrade();
            cx.defer(move |cx| {
                let _ = entity.update(cx, |this, cx| {
                    if this.loading.is_none() {
                        this.reload(cx)
                    }
                });
            });
        }
        let controls = mount
            .as_ref()
            .map(|mount| self.render_mcp_controls(context.0.clone(), mount.clone(), cx));
        let draft = self.draft(kind);
        let mut body = v_flex().w_full().min_w_0().gap_4();
        let add = cx.entity();
        body = body.child(
            h_flex().w_full().justify_end().child(
                Button::new("file-add")
                    .small()
                    .outline()
                    .icon(IconName::Plus)
                    .label(t(cx, "pi-files-add"))
                    .disabled(self.busy() || self.loading.is_some() || draft.names().is_err())
                    .on_click(move |_, window, cx| Self::open_editor(&add, kind, None, window, cx)),
            ),
        );
        if let Some(controls) = controls {
            body = body.child(controls);
        }
        if self.loading.is_some() {
            return body.child(t(cx, "pi-settings-loading")).into_any_element();
        }
        if let Some(error) = draft.error {
            body = body.child(
                div()
                    .text_color(cx.theme().danger)
                    .child(t(cx, error_key(error))),
            );
        }
        let Ok(names) = draft.names() else {
            return body
                .child(t(cx, "pi-resource-unsupported"))
                .into_any_element();
        };
        if draft.file.as_ref().is_some_and(File::comments) {
            body = body.child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(t(cx, "pi-files-comments")),
            );
        }
        let busy = self.busy();
        if names.is_empty() {
            body = body.child(
                div()
                    .py_2()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(t(cx, "pi-files-empty")),
            );
        }
        let mut mcp_rows = Vec::new();
        for name in names {
            let Some(entry) = self.draft(kind).entry(&name) else {
                continue;
            };
            let label = if kind == Kind::Mcp {
                if entry.partial() {
                    t(cx, "pi-files-override")
                } else {
                    entry
                        .visible(&["type".into()])
                        .and_then(|v| v.as_str().map(str::to_owned))
                        .unwrap_or_else(|| {
                            if entry.present(&["url".into()]) {
                                "http".into()
                            } else {
                                "stdio".into()
                            }
                        })
                }
            } else {
                let mut args = fluent_bundle::FluentArgs::new();
                match entry.len(&["models".into()]) {
                    Ok(count) => args.set("models", count as i64),
                    Err(_) => args.set("models", "—"),
                };
                match entry.keys(&["modelOverrides".into()]) {
                    Ok(keys) => args.set("overrides", keys.len() as i64),
                    Err(_) => args.set("overrides", "—"),
                };
                t_with_args(cx, "pi-files-provider-summary", &args)
            };
            let label = if kind == Kind::Mcp {
                format!("{label} · {}", self.mcp_row_status(&name, cx))
            } else {
                label
            };
            let label =
                if entry.disabled() && (kind != Kind::Mcp || !self.mcp_disabled_in_report(&name)) {
                    format!("{label} · {}", t(cx, "pi-files-disabled"))
                } else {
                    label
                };
            let has_auth = kind == Kind::Models && self.has_stored_credentials(&name);
            let e = cx.entity();
            let oauth = if kind == Kind::Mcp {
                self.mcp.file.as_ref().map(|file| {
                    file.mcp_oauth_eligibility(
                        &name,
                        self.global_mcp.as_ref().and_then(|f| f.as_ref().ok()),
                        &self.scope,
                        context.1,
                    )
                })
            } else {
                None
            };
            let command = context.0.clone();
            let mount = mount.clone();
            let cli_disabled = self.dirty() || busy || command.is_none();
            let edit = (e.clone(), name.clone());
            let delete = (e.clone(), name.clone());
            let menu_owner = e.clone();
            let menu_name = name.clone();
            let mut actions = h_flex().min_w_0().flex_wrap().gap_1();
            if kind == Kind::Mcp {
                actions = actions.child(self.render_enabled(
                    Some(&name),
                    entry.visible(&["enabled".into()]),
                    window,
                    cx,
                ));
            }
            if kind == Kind::Mcp
                && self.mcp_needs_auth(&name)
                && oauth == Some(gupi_resources::pi_mcp::Eligibility::Eligible)
                && let Some(login_mount) = mount.clone()
            {
                let (owner, login_name, login_command) = (e.clone(), name.clone(), command.clone());
                actions = actions.child(
                    Button::new(SharedString::from(format!("mcp-login-{name}")))
                        .small()
                        .outline()
                        .label(t(cx, "pi-mcp-login"))
                        .disabled(cli_disabled)
                        .on_click(move |_, window, cx| {
                            if let Some(command) = login_command.clone() {
                                FilePages::mcp_dialog(
                                    &owner,
                                    login_name.clone(),
                                    true,
                                    command,
                                    login_mount.clone(),
                                    window,
                                    cx,
                                );
                            }
                        }),
                );
            }
            actions = actions
                .child(
                    Button::new(SharedString::from(format!("file-row-{name}")))
                        .ghost()
                        .small()
                        .icon(IconName::SquarePen)
                        .tooltip(format!("{}: {name}", t(cx, "pi-files-edit")))
                        .accessibility_label(format!("{}: {name}", t(cx, "pi-files-edit")))
                        .disabled(busy)
                        .on_click(move |_, window, cx| {
                            Self::open_editor(&edit.0, kind, Some(edit.1.clone()), window, cx)
                        }),
                )
                .child(
                    Button::new(SharedString::from(format!("file-delete-{name}")))
                        .ghost()
                        .small()
                        .icon(IconName::Trash)
                        .tooltip(format!("{}: {name}", t(cx, "pi-files-delete")))
                        .accessibility_label(format!("{}: {name}", t(cx, "pi-files-delete")))
                        .disabled(busy)
                        .on_click(move |_, _, cx| {
                            delete.0.update(cx, |this, cx| {
                                this.draft_mut(kind).edit(delete.1.clone(), None);
                                cx.notify();
                            })
                        }),
                )
                .child(
                    Button::new(SharedString::from(format!("file-menu-{name}")))
                        .ghost()
                        .small()
                        .icon(IconName::Ellipsis)
                        .tooltip(format!("{}: {name}", t(cx, "pi-field-actions")))
                        .accessibility_label(format!("{}: {name}", t(cx, "pi-field-actions")))
                        .disabled(busy)
                        .dropdown_menu(move |menu, _, cx| {
                            if kind == Kind::Models {
                                let owner = menu_owner.clone();
                                menu.item(
                                    PopupMenuItem::new(t(cx, "pi-files-manage-auth")).on_click(
                                        move |_, _, cx| {
                                            owner.update(cx, |_, cx| {
                                                cx.emit(super::CredentialPage::Auth)
                                            })
                                        },
                                    ),
                                )
                            } else if let (Some(eligibility), Some(mount)) = (oauth, mount.as_ref())
                            {
                                Self::mcp_menu(
                                    menu,
                                    &menu_owner,
                                    &menu_name,
                                    eligibility,
                                    command.clone(),
                                    mount.clone(),
                                    cli_disabled,
                                    cx,
                                )
                            } else {
                                menu
                            }
                        }),
                );
            if kind == Kind::Models {
                let models = entry
                    .visible(&["models".into()])
                    .and_then(|models| models.as_array().cloned())
                    .unwrap_or_default();
                let model_names = models
                    .iter()
                    .filter_map(|model| {
                        model
                            .get("name")
                            .or_else(|| model.get("id"))
                            .and_then(Value::as_str)
                    })
                    .collect::<Vec<_>>()
                    .join("、");
                let mut metadata = vec![label];
                for key in ["baseUrl", "api"] {
                    if let Some(Value::String(value)) = entry.visible(&[key.into()]) {
                        metadata.push(value);
                    }
                }
                let info = v_flex()
                    .min_w_0()
                    .gap_1()
                    .child(
                        div()
                            .text_sm()
                            .debug_selector({
                                let name = name.clone();
                                move || format!("file-name-{name}")
                            })
                            .child(model_names),
                    )
                    .when(has_auth, |info| {
                        info.child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .debug_selector({
                                    let name = name.clone();
                                    move || format!("file-auth-{name}")
                                })
                                .child(t(cx, "pi-files-auth-stored")),
                        )
                    })
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(metadata.join(" · ")),
                    );
                let title = super::provider_name(&name, cx);
                body = body.child(
                    v_flex()
                        .w_full()
                        .min_w_0()
                        .gap_3()
                        .mt_4()
                        .child(div().font_weight(FontWeight::SEMIBOLD).child(title))
                        .child(
                            gpui_kit::component::group_box::GroupBox::new()
                                .fill()
                                .content_style(
                                    StyleRefinement::default()
                                        .w_full()
                                        .min_w_0()
                                        .px_5()
                                        .py_0()
                                        .gap_0(),
                                )
                                .child(crate::features::settings::composition::settings_row(
                                    &crate::features::settings::composition::PageLayout::new(
                                        width - window.rem_size() * 2.5,
                                        window.rem_size(),
                                    ),
                                    info,
                                    None,
                                    actions,
                                    false,
                                    cx,
                                )),
                        ),
                );
            } else {
                let connection = entry
                    .visible(&["url".into()])
                    .or_else(|| entry.visible(&["command".into()]))
                    .and_then(|value| value.as_str().map(str::to_owned))
                    .unwrap_or_default();
                mcp_rows.push(
                    gpui_kit::component::table::TableRow::new()
                        .child(
                            gpui_kit::component::table::TableCell::new()
                                .flex_1()
                                .min_w_0()
                                .whitespace_normal()
                                .py_4()
                                .child(
                                    v_flex()
                                        .min_w_0()
                                        .gap_1()
                                        .child(name)
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(cx.theme().muted_foreground)
                                                .child(connection),
                                        )
                                        .when(narrow, |info| {
                                            info.child(
                                                div()
                                                    .text_xs()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .child(label.clone()),
                                            )
                                        }),
                                ),
                        )
                        .when(!narrow, |row| {
                            row.child(
                                gpui_kit::component::table::TableCell::new()
                                    .flex_1()
                                    .min_w_0()
                                    .whitespace_normal()
                                    .child(label),
                            )
                        })
                        .child(
                            gpui_kit::component::table::TableCell::new()
                                .w(action_width)
                                .min_w_0()
                                .flex_none()
                                .child(actions),
                        ),
                );
            }
        }
        if kind == Kind::Models {
            body = body.child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(t(cx, "pi-files-startup-only")),
            );
        }
        if !mcp_rows.is_empty() {
            use gpui_kit::component::table::{Table, TableBody, TableHead, TableHeader, TableRow};
            body = body.child(
                Table::new()
                    .accessibility_label(t(cx, "pi-files-servers"))
                    .child(
                        TableHeader::new().child(
                            TableRow::new()
                                .child(
                                    TableHead::new()
                                        .flex_1()
                                        .min_w_0()
                                        .child(t(cx, "pi-files-servers")),
                                )
                                .when(!narrow, |row| {
                                    row.child(
                                        TableHead::new()
                                            .flex_1()
                                            .min_w_0()
                                            .child(t(cx, "settings-about-status")),
                                    )
                                })
                                .child(TableHead::new().w(action_width).min_w_0().flex_none()),
                        ),
                    )
                    .child(TableBody::new().children(mcp_rows)),
            );
        }
        if kind == Kind::Mcp {
            body = body.child(self.render_mcp_results(cx));
        }
        body.into_any_element()
    }
    fn set_auto(&mut self, value: Option<Value>, cx: &mut Context<Self>) {
        let original = self.mcp.file.as_ref().and_then(File::auto_enable);
        self.mcp.auto = if ordered_eq(original.as_ref(), value.as_ref()) {
            None
        } else {
            Some(value)
        };
        cx.notify();
    }
    fn render_auto(
        &mut self,
        layout: &crate::features::settings::composition::PageLayout,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let current = self
            .mcp
            .auto
            .clone()
            .unwrap_or_else(|| self.mcp.file.as_ref().and_then(File::auto_enable));
        let control = self.render_enabled(None, current, window, cx);
        let value =
            v_flex()
                .gap_1()
                .items_end()
                .child(control)
                .when(self.mcp.auto.is_some(), |value| {
                    value.child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(t(cx, "pi-unsaved")),
                    )
                });
        crate::features::settings::composition::settings_row(
            layout,
            t(cx, "pi-files-label-autoEnableCodemode"),
            None,
            value,
            false,
            cx,
        )
    }
    fn open_editor(
        entity: &Entity<Self>,
        kind: Kind,
        name: Option<String>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let this = entity.read(cx);
        let entry = name
            .as_deref()
            .and_then(|n| this.draft(kind).entry(n))
            .unwrap_or_else(|| Entry::new(kind));
        let project = kind == Kind::Mcp && matches!(this.scope, Scope::Project(_));
        let editing = name.is_some();
        let editor = cx.new(|cx| {
            EntryEditor::new(
                kind,
                name.unwrap_or_default(),
                entry,
                editing,
                project,
                window,
                cx,
            )
        });
        let parent = entity.clone();
        window.open_dialog(cx, move |dialog, window, cx| {
            let editor = editor.clone();
            let save = (parent.clone(), editor.clone());
            dialog
                .title(t(
                    cx,
                    if kind == Kind::Models {
                        "pi-files-models"
                    } else {
                        "pi-files-mcp"
                    },
                ))
                .width(window.rem_size() * 42.)
                .overlay_closable(false)
                .content(move |content, _, _| content.min_h_0().child(editor.clone()))
                .footer(super::dialog_buttons(
                    "pi-files-stage",
                    false,
                    false,
                    crate::features::settings::DialogCommit::Primary,
                    cx,
                ))
                .on_ok(move |_, window, cx| {
                    let result = save.1.update(cx, |editor, cx| editor.prepare(cx));
                    let Ok((name, entry)) = result else {
                        return false;
                    };
                    let check = save.0.read(cx);
                    let result = if editing {
                        Ok(())
                    } else {
                        check.check_new(kind, &name, &entry)
                    };
                    if let Err(e) = result {
                        save.1.update(cx, |editor, cx| editor.show_error(e, cx));
                        return false;
                    }
                    save.0.update(cx, |this, cx| {
                        this.draft_mut(kind).edit(name, Some(entry));
                        cx.notify()
                    });
                    let _ = window;
                    true
                })
        });
    }
    fn check_new(&self, kind: Kind, name: &str, entry: &Entry) -> Result<(), Error> {
        let draft = self.draft(kind);
        let file = draft.file.as_ref().ok_or(Error::Read)?;
        let global = match &self.global_mcp {
            Some(Ok(file)) => Some(file),
            Some(Err(error)) => return Err(*error),
            None => None,
        };
        file.check_name(name, global)?;
        let names = draft.names()?;
        if names.iter().any(|n| {
            n == name || (kind == Kind::Mcp && n.replace('-', "_") == name.replace('-', "_"))
        }) {
            return Err(Error::Duplicate);
        }
        if kind == Kind::Mcp
            && matches!(self.scope, Scope::Project(_))
            && entry.partial()
            && !global.is_some_and(|f| f.entry(name).is_some())
        {
            return Err(Error::Invalid);
        }
        Ok(())
    }
}
fn error_key(error: Error) -> &'static str {
    match error {
        Error::Conflict => "pi-settings-conflict",
        Error::Duplicate => "pi-files-duplicate",
        _ => "pi-files-invalid",
    }
}

#[cfg(test)]
mod tests {
    use super::{Draft, Entry, File, Kind};
    use serde_json::json;
    #[test]
    fn staged_edits_track_order_and_keep_unchanged_entries_out_of_commit() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mcp.json");
        std::fs::write(&path,r#"{"mcpServers":{"a":{"command":"x","toolExposure":{"first":"direct","second":"hidden"}}}}"#).unwrap();
        let file = File::load(Kind::Mcp, &path).unwrap();
        let entry = file.entry("a").unwrap();
        let mut draft = Draft {
            file: Some(file),
            ..Default::default()
        };
        draft.edit("a".into(), Some(entry.clone()));
        assert!(!draft.dirty());
        let mut reordered = entry.clone();
        reordered.move_item(&["toolExposure".into()], 0, 1).unwrap();
        draft.edit("a".into(), Some(reordered));
        assert!(draft.dirty());
        draft.edit("a".into(), Some(entry));
        assert!(!draft.dirty());
        let mut new = Entry::new(Kind::Mcp);
        new.set(&["command".into()], Some(json!("new"))).unwrap();
        draft.edit("b".into(), Some(new));
        assert_eq!(draft.names().unwrap(), ["a", "b"]);
        draft.edit("b".into(), None);
        assert_eq!(draft.names().unwrap(), ["a"]);
        assert!(!draft.dirty());
    }
    struct Mounted {
        files: gpui_kit::Entity<super::FilePages>,
        kind: Kind,
    }
    impl gpui_kit::Render for Mounted {
        fn render(
            &mut self,
            window: &mut gpui_kit::Window,
            cx: &mut gpui_kit::Context<Self>,
        ) -> impl gpui_kit::IntoElement {
            self.files
                .update(cx, |files, cx| files.render_page(self.kind, window, cx))
        }
    }
    struct AutoMounted(gpui_kit::Entity<super::FilePages>);
    impl gpui_kit::Render for AutoMounted {
        fn render(
            &mut self,
            window: &mut gpui_kit::Window,
            cx: &mut gpui_kit::Context<Self>,
        ) -> impl gpui_kit::IntoElement {
            use gpui_kit::{ParentElement as _, Styled as _};
            gpui_kit::div()
                .w_full()
                .child(self.0.update(cx, |files, cx| {
                    files.render_auto(
                        &crate::features::settings::composition::PageLayout::new(
                            gpui_kit::px(800.),
                            window.rem_size(),
                        ),
                        window,
                        cx,
                    )
                }))
        }
    }
    #[gpui_kit::test]
    fn codemode_select_preserves_unknown_and_stages_explicit_choices(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        use gpui_kit::AppContext as _;
        use gpui_kit::test::TestWindowExt as _;
        cx.update(|cx| {
            gpui_kit::init(cx);
            gupi_settings::i18n::apply(gupi_settings::config::AppLanguage::German, cx);
        });
        let agent = tempfile::tempdir().unwrap();
        let path = agent.path().join("mcp.json");
        let original = r#"{"autoEnableCodemode":"future","mcpServers":{}}"#;
        std::fs::write(&path, original).unwrap();
        let mut owner = None;
        let (_, cx) = cx.add_window_view(|window, cx| {
            let files = cx.new(|cx| {
                let mut files = super::FilePages::new(cx);
                files.agent = Ok(agent.path().to_owned());
                files.mcp.file = Some(File::load(Kind::Mcp, &path).unwrap());
                files
            });
            owner = Some(files.clone());
            let mounted = cx.new(|_| AutoMounted(files));
            gpui_kit::component::Root::new(mounted, window, cx)
        });
        let owner = owner.unwrap();
        cx.simulate_resize(gpui_kit::size(gpui_kit::px(800.), gpui_kit::px(600.)));
        cx.update(|window, cx| window.render_frame(cx));
        let picker = owner.read_with(cx, |files, _| files.enabled_inputs["auto"].state.clone());
        owner.read_with(cx, |files, cx| {
            assert_eq!(files.mcp.auto, None);
            assert_eq!(picker.read(cx).selected_value(), None);
        });
        for expected in [None, Some(true), Some(false), None] {
            picker.update(cx, |_, cx| {
                cx.emit(gpui_kit::component::select::SelectEvent::<
                    gpui_kit::component::searchable_list::SearchableVec<super::EnabledChoice>,
                >::Confirm(Some(expected)));
            });
            cx.run_until_parked();
            owner.read_with(cx, |files, _| {
                assert_eq!(files.mcp.auto, Some(expected.map(serde_json::Value::Bool)))
            });
        }
        assert_eq!(std::fs::read_to_string(path).unwrap(), original);
    }
    #[gpui_kit::test]
    fn add_entry_button_opens_both_editors_without_reentering_owner(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        use gpui_kit::AppContext as _;
        use gpui_kit::component::WindowExt as _;
        use gpui_kit::test::TestWindowExt as _;
        cx.update(|cx| {
            gpui_kit::init(cx);
            gupi_settings::i18n::apply(gupi_settings::config::AppLanguage::English, cx);
        });
        let agent = tempfile::tempdir().unwrap();
        for kind in [Kind::Models, Kind::Mcp] {
            let (_, cx) = cx.add_window_view(|window, cx| {
                let files = cx.new(|cx| {
                    let mut files = super::FilePages::new(cx);
                    files.agent = Ok(agent.path().to_owned());
                    files.draft_mut(kind).file = Some(
                        File::load(
                            kind,
                            &kind.file(agent.path(), &gupi_resources::pi_settings::Scope::Global),
                        )
                        .unwrap(),
                    );
                    files
                });
                let mounted = cx.new(|_| Mounted { files, kind });
                gpui_kit::component::Root::new(mounted, window, cx)
            });
            cx.update(|window, cx| {
                window.render_frame(cx);
                window.click("file-add", cx);
                window.render_frame(cx);
                assert!(window.has_active_dialog(cx));
            });
        }
    }
}

#[cfg(test)]
mod auth_metadata_tests;
