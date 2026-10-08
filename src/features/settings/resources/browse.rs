use super::packages::Mount;
use super::packages::region;
use super::*;
use gpui_kit::component::Selectable;
use gpui_kit::component::link::Link;
use gpui_kit::component::menu::DropdownMenu;
use gpui_kit::component::menu::PopupMenuItem;
use gpui_kit::component::resizable::ResizableState;
use gpui_kit::component::resizable::h_resizable;
use gpui_kit::component::resizable::resizable_panel;
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::tag::Tag;
use gpui_tokio::Tokio;
use gupi_resources::catalog;
use gupi_resources::catalog::Declared;
use gupi_resources::catalog::Details;
use gupi_resources::catalog::Problem;
use gupi_resources::catalog::SearchPage;
use std::time::Duration;

/// Below this body width the browse view shows the list or one package.
const SPLIT_MIN_WIDTH: f32 = 640.;
/// Below this body width the list footer uses its shortest form.
const COMPACT_FOOTER_WIDTH: f32 = 420.;

enum DetailState {
    Loading(#[allow(dead_code)] Task<()>),
    Ready(Details),
    Failed(Problem),
}

struct FailedSearch {
    query: String,
    from: usize,
    problem: Problem,
}

/// Catalog browsing owned by the settings window; nothing is persisted.
pub(super) struct Browse {
    query: Entity<InputState>,
    /// Identity of the latest search; older responses are ignored.
    serial: u64,
    page: Option<SearchPage>,
    problem: Option<FailedSearch>,
    searching: Option<Task<()>>,
    debounce: Option<Task<()>>,
    selected: Option<String>,
    details: std::collections::BTreeMap<String, DetailState>,
    /// In a narrow body, whether the selected package replaces the list.
    showing_detail: bool,
    /// Divider position between the result list and the package details.
    split: Entity<ResizableState>,
    list_scroll: ScrollHandle,
    detail_scroll: ScrollHandle,
}
impl Browse {
    pub fn new(window: &mut Window, cx: &mut Context<ResourcesView>) -> (Self, Subscription) {
        let query = cx.new(|cx| InputState::new(window, cx).placeholder(t(cx, "packages-search")));
        let subscription = cx.subscribe(&query, |this: &mut ResourcesView, _, event, cx| {
            if matches!(event, InputEvent::Change) {
                this.schedule_search(cx);
            }
        });
        (
            Self {
                query,
                serial: 0,
                page: None,
                problem: None,
                searching: None,
                debounce: None,
                selected: None,
                details: Default::default(),
                showing_detail: false,
                split: cx.new(|_| ResizableState::default()),
                list_scroll: ScrollHandle::new(),
                detail_scroll: ScrollHandle::new(),
            },
            subscription,
        )
    }
}

impl ResourcesView {
    fn schedule_search(&mut self, cx: &mut Context<Self>) {
        // Wait for typing to pause before asking the registry (Zed uses 250 ms).
        self.browse.debounce = Some(cx.spawn(async move |owner, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(250))
                .await;
            let _ = owner.update(cx, |this, cx| this.search_catalog(0, cx));
        }));
    }

    pub(super) fn search_catalog(&mut self, from: usize, cx: &mut Context<Self>) {
        let query = self.browse.query.read(cx).value().to_string();
        self.start_search(query, from, cx);
    }

    fn start_search(&mut self, query: String, from: usize, cx: &mut Context<Self>) {
        self.browse.debounce = None;
        self.browse.problem = None;
        self.browse.serial += 1;
        let serial = self.browse.serial;
        let request = Tokio::spawn(cx, catalog::search(query.clone(), from));
        self.browse.searching = Some(cx.spawn(async move |owner, cx| {
            let result = request.await.unwrap_or(Err(Problem::Network));
            let _ = owner.update(cx, |this, cx| {
                if this.browse.serial != serial {
                    return;
                }
                this.browse.searching = None;
                match result {
                    Ok(page) => {
                        this.browse.page = Some(page);
                        this.browse.problem = None;
                    }
                    // Keep the previous results visible next to the error.
                    Err(problem) => {
                        this.browse.problem = Some(FailedSearch {
                            query,
                            from,
                            problem,
                        });
                    }
                }
                cx.notify();
            });
        }));
        cx.notify();
    }

    fn select_package(&mut self, name: String, cx: &mut Context<Self>) {
        if self.browse.selected.as_ref() != Some(&name) {
            self.browse.detail_scroll = ScrollHandle::new();
        }
        self.browse.selected = Some(name.clone());
        self.browse.showing_detail = true;
        if !matches!(
            self.browse.details.get(&name),
            Some(DetailState::Ready(_) | DetailState::Loading(_))
        ) {
            self.load_details(name, cx);
        }
        cx.notify();
    }

    fn load_details(&mut self, name: String, cx: &mut Context<Self>) {
        let request = Tokio::spawn(cx, catalog::details(name.clone()));
        let target = name.clone();
        let task = cx.spawn(async move |owner, cx| {
            let result = request.await.unwrap_or(Err(Problem::Network));
            let _ = owner.update(cx, |this, cx| {
                // Details are keyed by package; a response never changes the selection.
                this.browse.details.insert(
                    target,
                    match result {
                        Ok(details) => DetailState::Ready(details),
                        Err(problem) => DetailState::Failed(problem),
                    },
                );
                cx.notify();
            });
        });
        self.browse.details.insert(name, DetailState::Loading(task));
        cx.notify();
    }

    /// The installed package for a registry name, matched by Pi's npm identity.
    fn installed_package<'a>(&self, name: &str, cx: &'a App) -> Option<&'a io::Package> {
        self.controller
            .read(cx)
            .catalog()
            .data()?
            .packages
            .iter()
            .find(|package| catalog::installed_name(&package.source) == Some(name))
    }

    /// Search row above either the full-width result list or, once a package
    /// is selected, a draggable split (narrow bodies show one side at a time).
    pub(super) fn render_browse(
        &mut self,
        width: Pixels,
        mount: Mount,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if self.browse.page.is_none()
            && self.browse.problem.is_none()
            && self.browse.searching.is_none()
        {
            let owner = cx.entity().downgrade();
            cx.defer(move |cx| {
                let _ = owner.update(cx, |this, cx| {
                    if this.browse.searching.is_none() && this.browse.page.is_none() {
                        this.search_catalog(0, cx);
                    }
                });
            });
        }
        let width = width.as_f32();
        let split = width >= SPLIT_MIN_WIDTH;
        let compact = width < COMPACT_FOOTER_WIDTH;
        let search = Input::new(&self.browse.query)
            .prefix(gpui_kit::component::Icon::new(IconName::Search))
            .when(self.browse.searching.is_some(), |input| {
                input.suffix(Spinner::new().small())
            });
        let selected = self.browse.selected.clone();
        let body = match selected {
            Some(name) if split => h_resizable("packages-browse")
                .with_state(&self.browse.split)
                .child(
                    resizable_panel()
                        .size(px((width * 0.45).max(280.)))
                        .size_range(px(260.)..px((width - 320.).max(260.)))
                        .child(self.render_list_pane(compact, mount, cx)),
                )
                .child(resizable_panel().child(self.render_detail_pane(&name, false, mount, cx)))
                .into_any_element(),
            Some(name) if self.browse.showing_detail => {
                self.render_detail_pane(&name, true, mount, cx)
            }
            _ => self.render_list_pane(compact, mount, cx),
        };
        v_flex()
            .size_full()
            .min_h_0()
            .gap_3()
            .child(search)
            .child(
                div()
                    .when(mount == Mount::Bounded, |body| body.flex_1().min_h_0())
                    .child(body),
            )
            .into_any_element()
    }

    /// The result list owns its scrolling; source and paging share one footer.
    fn render_list_pane(&self, compact: bool, mount: Mount, cx: &mut Context<Self>) -> AnyElement {
        v_flex()
            .size_full()
            .min_h_0()
            .child(
                region("packages-results", &self.browse.list_scroll, mount)
                    .pr_2()
                    .child(self.render_results(cx)),
            )
            .child(self.render_list_footer(compact, cx))
            .into_any_element()
    }

    fn render_list_footer(&self, compact: bool, cx: &mut Context<Self>) -> AnyElement {
        let source = h_flex()
            .id("packages-source")
            .min_w_0()
            .gap_2()
            .text_sm()
            .text_color(cx.theme().muted_foreground)
            .child(div().truncate().child(t(cx, "packages-source-short")))
            .tooltip({
                let text = t(cx, "packages-source-npm");
                move |window, cx| {
                    gpui_kit::component::tooltip::Tooltip::new(text.clone()).build(window, cx)
                }
            })
            .child(
                Link::new("packages-directory")
                    .href(catalog::DIRECTORY_URL)
                    .flex_shrink_0()
                    .child(t(cx, "packages-directory")),
            );
        let pager = self.browse.page.as_ref().map(|page| {
            let mut args = fluent_bundle::FluentArgs::new();
            args.set("page", page.number() as i64);
            let from = page.from;
            let has_next = page.has_next();
            let busy = self.browse.searching.is_some();
            h_flex()
                .flex_shrink_0()
                .gap_1()
                .items_center()
                .child(
                    Button::new("packages-previous")
                        .ghost()
                        .xsmall()
                        .icon(IconName::ChevronLeft)
                        .tooltip(t(cx, "packages-previous"))
                        .accessibility_label(t(cx, "packages-previous"))
                        .disabled(busy || from == 0)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.search_catalog(from.saturating_sub(catalog::PAGE_SIZE), cx)
                        })),
                )
                .when(!compact, |row| {
                    row.child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(gupi_settings::i18n::t_with_args(cx, "packages-page", &args)),
                    )
                })
                .child(
                    Button::new("packages-next")
                        .ghost()
                        .xsmall()
                        .icon(IconName::ChevronRight)
                        .tooltip(t(cx, "packages-next"))
                        .accessibility_label(t(cx, "packages-next"))
                        .disabled(busy || !has_next)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.search_catalog(from + catalog::PAGE_SIZE, cx)
                        })),
                )
        });
        h_flex()
            .flex_shrink_0()
            .pt_2()
            .gap_3()
            .items_center()
            .justify_between()
            .border_t_1()
            .border_color(cx.theme().border)
            .child(source)
            .children(pager)
            .into_any_element()
    }

    /// Package details with their own scrolling; Close (split) or Back (narrow).
    fn render_detail_pane(
        &self,
        name: &str,
        narrow: bool,
        mount: Mount,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let header = if narrow {
            h_flex().child(
                Button::new("packages-back")
                    .ghost()
                    .small()
                    .icon(IconName::ChevronLeft)
                    .label(t(cx, "packages-back"))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.browse.showing_detail = false;
                        cx.notify();
                    })),
            )
        } else {
            h_flex()
                .gap_2()
                .items_center()
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(name.to_owned()),
                )
                .child(
                    Button::new("packages-close")
                        .ghost()
                        .small()
                        .icon(IconName::X)
                        .tooltip(t(cx, "packages-close"))
                        .accessibility_label(t(cx, "packages-close"))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.browse.selected = None;
                            this.browse.showing_detail = false;
                            cx.notify();
                        })),
                )
        };
        v_flex()
            .size_full()
            .min_h_0()
            .when(!narrow, |pane| pane.pl_4())
            .gap_3()
            .child(header)
            .child(
                region("packages-detail", &self.browse.detail_scroll, mount)
                    .child(self.render_package_detail(name, narrow, cx)),
            )
            .into_any_element()
    }

    fn render_results(&self, cx: &mut Context<Self>) -> AnyElement {
        let mut view = v_flex().gap_1();
        if let Some(failure) = &self.browse.problem {
            view = view.child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().danger)
                            .child(t(cx, failure.problem.key())),
                    )
                    .child(
                        Button::new("packages-retry")
                            .small()
                            .outline()
                            .label(t(cx, "packages-retry"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                if let Some(failure) = this.browse.problem.take() {
                                    this.start_search(failure.query, failure.from, cx);
                                }
                            })),
                    ),
            );
        }
        let Some(page) = &self.browse.page else {
            if self.browse.searching.is_some() {
                view = view.child(
                    h_flex()
                        .gap_2()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(Spinner::new().small())
                        .child(t(cx, "packages-loading")),
                );
            }
            return view.into_any_element();
        };
        if page.entries.is_empty() {
            view = view.child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(t(cx, "packages-page-empty")),
            );
        }

        for entry in &page.entries {
            let selected = self.browse.selected.as_deref() == Some(entry.name.as_str());
            let installed = self.installed_package(&entry.name, cx).is_some();
            let name = entry.name.clone();
            view = view.child(
                Button::new(SharedString::from(format!("package-entry-{}", entry.name)))
                    .ghost()
                    .selected(selected)
                    .w_full()
                    .h_auto()
                    .py_2()
                    .justify_start()
                    .on_click(
                        cx.listener(move |this, _, _, cx| this.select_package(name.clone(), cx)),
                    )
                    .child(
                        v_flex()
                            .w_full()
                            .min_w_0()
                            .gap_1()
                            .child(
                                h_flex()
                                    .w_full()
                                    .gap_2()
                                    .child(
                                        div()
                                            .min_w_0()
                                            .flex_1()
                                            .text_left()
                                            .truncate()
                                            .font_weight(FontWeight::MEDIUM)
                                            .child(entry.name.clone()),
                                    )
                                    .child(
                                        div()
                                            .flex_shrink_0()
                                            .text_sm()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(entry.version.clone()),
                                    )
                                    .when(installed, |row| {
                                        row.child(
                                            Tag::secondary()
                                                .small()
                                                .outline()
                                                .child(t(cx, "packages-installed")),
                                        )
                                    }),
                            )
                            .children(entry.description.clone().map(|description| {
                                div()
                                    .w_full()
                                    .text_left()
                                    .text_sm()
                                    .truncate()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(description)
                            })),
                    ),
            );
        }
        view.into_any_element()
    }

    fn render_package_detail(
        &self,
        name: &str,
        narrow: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let entry = self
            .browse
            .page
            .as_ref()
            .and_then(|page| page.entries.iter().find(|entry| entry.name == name))
            .cloned();
        let details = self.browse.details.get(name);
        let source = catalog::source(name);
        let mut view = v_flex().gap_3().min_w_0();
        let ready = match details {
            Some(DetailState::Ready(details)) => Some(details),
            _ => None,
        };
        let description = ready
            .and_then(|d| d.description.clone())
            .or_else(|| entry.as_ref().and_then(|e| e.description.clone()));
        let latest = ready
            .map(|d| d.version.clone())
            .or_else(|| entry.as_ref().map(|e| e.version.clone()));
        view = view
            .when(narrow, |view| {
                view.child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(name.to_owned()),
                )
            })
            .children(description.map(|text| div().text_sm().child(text)));
        let mut facts = Vec::new();
        if let Some(version) = &latest {
            let mut args = fluent_bundle::FluentArgs::new();
            args.set("version", version.clone());
            facts.push(gupi_settings::i18n::t_with_args(
                cx,
                "packages-latest",
                &args,
            ));
        }
        if let Some(license) = ready.and_then(|d| d.license.clone()) {
            facts.push(license);
        }
        if let Some(publisher) = entry.as_ref().and_then(|e| e.publisher.clone()) {
            let mut args = fluent_bundle::FluentArgs::new();
            args.set("publisher", publisher);
            facts.push(gupi_settings::i18n::t_with_args(
                cx,
                "packages-publisher",
                &args,
            ));
        }
        if !facts.is_empty() {
            view = view.child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(facts.join(" · ")),
            );
        }
        let repository = ready
            .and_then(|d| d.repository.clone())
            .or_else(|| entry.as_ref().and_then(|e| e.repository.clone()));
        let npm = entry.as_ref().and_then(|e| e.npm.clone());
        if repository.is_some() || npm.is_some() {
            view = view.child(
                h_flex()
                    .gap_3()
                    .text_sm()
                    .children(repository.clone().map(|url| {
                        Link::new("package-repository")
                            .href(url)
                            .child(t(cx, "packages-repository"))
                    }))
                    .children(npm.clone().map(|url| {
                        Link::new("package-npm")
                            .href(url)
                            .child(t(cx, "packages-npm"))
                    })),
            );
        }
        view = view.child(self.render_declared(details, cx));
        view = view.child(
            v_flex().gap_1().text_sm().child(
                h_flex()
                    .gap_2()
                    .child(
                        div()
                            .text_color(cx.theme().muted_foreground)
                            .child(t(cx, "packages-install-source")),
                    )
                    .child(source.clone()),
            ),
        );
        view = view.child(self.render_detail_actions(name, latest, repository, npm, cx));
        view.into_any_element()
    }

    fn render_declared(&self, details: Option<&DetailState>, cx: &mut Context<Self>) -> AnyElement {
        let heading = div()
            .text_sm()
            .font_weight(FontWeight::MEDIUM)
            .child(t(cx, "packages-declared"));
        let muted = |text: String| {
            div()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(text)
        };
        let body =
            match details {
                None | Some(DetailState::Loading(_)) => h_flex()
                    .gap_2()
                    .child(Spinner::new().small())
                    .child(muted(t(cx, "packages-loading")))
                    .into_any_element(),
                Some(DetailState::Failed(problem)) => {
                    let name = self.browse.selected.clone();
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().danger)
                                .child(t(cx, problem.key())),
                        )
                        .child(
                            Button::new("package-details-retry")
                                .small()
                                .outline()
                                .label(t(cx, "packages-retry"))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    if let Some(name) = name.clone() {
                                        this.load_details(name, cx);
                                    }
                                })),
                        )
                        .into_any_element()
                }
                Some(DetailState::Ready(details)) => {
                    match &details.declared {
                        None => muted(t(cx, "packages-undeclared")).into_any_element(),
                        Some(declared) => v_flex()
                            .gap_1()
                            .children(declared.iter().map(|(kind, paths)| {
                                h_flex()
                                    .gap_3()
                                    .items_start()
                                    .text_sm()
                                    .child(
                                        div()
                                            .w(px(96.))
                                            .flex_shrink_0()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(t(cx, declared_key(*kind))),
                                    )
                                    .child(v_flex().min_w_0().children(paths.iter().map(|path| {
                                        div().min_w_0().truncate().child(path.clone())
                                    })))
                            }))
                            .into_any_element(),
                    }
                }
            };
        v_flex()
            .gap_1()
            .child(heading)
            .child(body)
            .into_any_element()
    }

    fn render_detail_actions(
        &self,
        name: &str,
        latest: Option<String>,
        repository: Option<String>,
        npm: Option<String>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let configured = self.config.read(cx).preferences(cx).pi_command;
        let pi_ready = self
            .applied_pi
            .read(cx)
            .ready_for(configured.as_deref())
            .is_some();
        let installed = self.installed_package(name, cx).cloned();
        let source = installed
            .as_ref()
            .map(|package| package.source.clone())
            .unwrap_or_else(|| catalog::source(name));
        let mutation = self.controller.read(cx).mutation();
        let updating = mutation.package_running(&source, "update");
        let installing = mutation.package_running(&source, "install");
        let removing = mutation.package_running(&source, "remove");
        let mut view = v_flex().gap_2();
        if let Some(package) = &installed {
            let mut state = h_flex().gap_2().items_center().child(
                Tag::secondary()
                    .small()
                    .outline()
                    .child(t(cx, "packages-installed")),
            );
            if !package.path.exists() {
                state = state.child(
                    Tag::danger()
                        .small()
                        .outline()
                        .child(t(cx, "packages-files-missing")),
                );
            } else if let Some(version) = &package.version {
                let mut args = fluent_bundle::FluentArgs::new();
                args.set("version", version.clone());
                state = state.child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(gupi_settings::i18n::t_with_args(
                            cx,
                            "packages-installed-version",
                            &args,
                        )),
                );
            }
            view = view.child(state);
            if let Some(pinned) = catalog::pinned_version(&package.source) {
                let mut args = fluent_bundle::FluentArgs::new();
                args.set("pinned", pinned.to_owned());
                args.set("version", pinned.to_owned());
                let key = match &latest {
                    Some(latest) => {
                        args.set("latest", latest.clone());
                        "packages-pinned"
                    }
                    None => "packages-pinned-installed",
                };
                view = view.child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(gupi_settings::i18n::t_with_args(cx, key, &args)),
                );
            }
        }
        view = view.child(
            div()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(t(cx, "packages-trust")),
        );
        if self.controller.read(cx).catalog().data().is_none() {
            view = view.child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(t(
                        cx,
                        if self.controller.read(cx).catalog().is_running() {
                            "settings-resource-loading"
                        } else {
                            "packages-local-unavailable"
                        },
                    )),
            );
            if let Some(error) = self.controller.read(cx).catalog().problem() {
                view = view.child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().danger)
                        .child(error.to_string()),
                );
            }
        }
        if !pi_ready {
            view = view.child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(t(cx, "settings-resource-pi-required")),
                    )
                    .child(
                        Button::new("packages-open-pi")
                            .small()
                            .outline()
                            .label(t(cx, "settings-open-pi"))
                            .on_click(cx.listener(|_, _, _, cx| cx.emit(Navigate::Pi))),
                    ),
            );
        }
        if let Some(result) = self.package_results.get(&source) {
            view = view.child(match result {
                Ok(()) => div()
                    .text_sm()
                    .child(t(cx, "packages-done"))
                    .into_any_element(),
                Err(error) => v_flex()
                    .gap_1()
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().danger)
                            .child(t(cx, "packages-failed")),
                    )
                    .child(
                        div()
                            .id("package-output")
                            .max_h(px(160.))
                            .overflow_y_scroll()
                            .text_xs()
                            .font_family(cx.theme().mono_font_family.clone())
                            .child(error.to_string()),
                    )
                    .into_any_element(),
            });
        }
        let disabled = !self.package_available(cx);
        let actions = if installed.is_some() {
            let update = source.clone();
            let remove = source.clone();
            h_flex()
                .gap_2()
                .child(
                    Button::new("package-detail-update")
                        .small()
                        .label(t(cx, "settings-package-update"))
                        .loading(updating)
                        .disabled(disabled)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.package("update", update.clone(), cx)
                        })),
                )
                .child(
                    Button::new("package-detail-remove")
                        .small()
                        .label(t(cx, "settings-package-remove-ellipsis"))
                        .loading(removing)
                        .disabled(disabled)
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.confirm_remove(None, remove.clone(), window, cx)
                        })),
                )
        } else {
            let install = source.clone();
            h_flex().gap_2().child(
                Button::new("package-detail-install")
                    .small()
                    .primary()
                    .label(t(cx, "packages-install"))
                    .loading(installing)
                    .disabled(disabled)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.package("install", install.clone(), cx)
                    })),
            )
        };
        let copy = source.clone();
        let actions = actions.child(
            Button::new("package-detail-more")
                .ghost()
                .small()
                .icon(IconName::Ellipsis)
                .tooltip(t(cx, "settings-resource-actions"))
                .accessibility_label(t(cx, "settings-resource-actions"))
                .dropdown_menu_with_anchor(Anchor::TopRight, move |menu, _, cx| {
                    let copy = copy.clone();
                    let mut menu = menu.item(
                        PopupMenuItem::new(t(cx, "packages-copy-source"))
                            .icon(IconName::Copy)
                            .on_click(move |_, _, cx| {
                                cx.write_to_clipboard(ClipboardItem::new_string(copy.clone()))
                            }),
                    );
                    if let Some(url) = repository.clone() {
                        menu = menu.item(
                            PopupMenuItem::new(t(cx, "packages-open-repository"))
                                .icon(IconName::ExternalLink)
                                .on_click(move |_, _, cx| cx.open_url(&url)),
                        );
                    }
                    if let Some(url) = npm.clone() {
                        menu = menu.item(
                            PopupMenuItem::new(t(cx, "packages-open-npm"))
                                .icon(IconName::ExternalLink)
                                .on_click(move |_, _, cx| cx.open_url(&url)),
                        );
                    }
                    menu
                }),
        );
        view.child(actions).into_any_element()
    }
}

fn declared_key(kind: Declared) -> &'static str {
    match kind {
        Declared::Extensions => "settings-package-kind-extensions",
        Declared::Skills => "settings-package-kind-skills",
        Declared::Prompts => "settings-package-kind-prompts",
        Declared::Themes => "settings-package-kind-themes",
    }
}
