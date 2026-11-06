use super::packages::{package_detail, package_panes, package_row};
use super::*;
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::component::link::Link;
use gpui_kit::component::menu::DropdownMenu;
use gpui_kit::component::menu::PopupMenuItem;
use gpui_kit::component::resizable::ResizableState;
use gpui_kit::component::scroll::ScrollableElement as _;
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
    focus: FocusHandle,
}
impl Browse {
    pub(super) fn search_input(&self) -> Input {
        Input::new(&self.query).small()
    }

    pub(super) fn reset_scope(&mut self, cx: &mut App) {
        self.selected = None;
        self.showing_detail = false;
        self.list_scroll = ScrollHandle::new();
        self.detail_scroll = ScrollHandle::new();
        self.split = cx.new(|_| ResizableState::default());
    }

    pub(super) fn refresh_language(&self, window: &mut Window, cx: &mut Context<ResourcesView>) {
        let placeholder = t(cx, "packages-search");
        self.query.update(cx, |input, cx| {
            input.set_placeholder(placeholder, window, cx);
        });
    }

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
                focus: cx.focus_handle().tab_stop(true),
            },
            subscription,
        )
    }
}

impl ResourcesView {
    fn schedule_search(&mut self, cx: &mut Context<Self>) {
        // Invalidate the old query immediately, including its in-flight response.
        self.browse.list_scroll.set_offset(Point::default());
        self.browse.serial += 1;
        self.browse.searching = None;
        self.browse.page = None;
        self.browse.problem = None;
        self.browse.selected = None;
        self.browse.showing_detail = false;
        // Wait for typing to pause before asking the registry (Zed uses 250 ms).
        self.browse.debounce = Some(cx.spawn(async move |owner, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(250))
                .await;
            let _ = owner.update(cx, |this, cx| this.search_catalog(0, cx));
        }));
        cx.notify();
    }

    pub(super) fn search_catalog(&mut self, from: usize, cx: &mut Context<Self>) {
        let query = self.browse.query.read(cx).value().to_string();
        self.start_search(query, from, cx);
    }

    fn start_search(&mut self, query: String, from: usize, cx: &mut Context<Self>) {
        self.browse.debounce = None;
        self.browse.problem = None;
        self.browse.page = None;
        self.browse.list_scroll.set_offset(Point::default());
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
        if self.browse.selected.as_deref() != Some(&name) {
            self.browse.detail_scroll.set_offset(Point::default());
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
        self.active()
            .read(cx)
            .catalog()
            .data()?
            .packages
            .iter()
            .find(|package| catalog::installed_name(&package.source) == Some(name))
    }

    /// Search row above either the full-width result list or, once a package
    /// is selected, a draggable split (narrow bodies show one side at a time).
    pub(super) fn render_browse(&mut self, width: Pixels, cx: &mut Context<Self>) -> AnyElement {
        if self.browse.page.is_none()
            && self.browse.problem.is_none()
            && self.browse.searching.is_none()
            && self.browse.debounce.is_none()
        {
            let owner = cx.entity().downgrade();
            cx.defer(move |cx| {
                let _ = owner.update(cx, |this, cx| {
                    if this.browse.searching.is_none()
                        && this.browse.debounce.is_none()
                        && this.browse.page.is_none()
                        && this.browse.problem.is_none()
                    {
                        this.search_catalog(0, cx);
                    }
                });
            });
        }
        let compact = width < px(COMPACT_FOOTER_WIDTH);
        let list = self.render_list_pane(compact, cx);
        let detail = self
            .browse
            .selected
            .clone()
            .map(|name| self.render_detail_pane(&name, width < px(640.), cx));
        package_panes(
            "packages-browse",
            width,
            &self.browse.split,
            list,
            detail,
            self.browse.showing_detail,
        )
    }

    /// Only results scroll; pagination remains at the list boundary.
    fn render_list_pane(&self, compact: bool, cx: &mut Context<Self>) -> AnyElement {
        v_flex()
            .size_full()
            .min_h_0()
            .child(
                div()
                    .id("packages-results")
                    .test_support()
                    .debug_selector(|| "packages-results".into())
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .overflow_y_scroll()
                    .track_scroll(&self.browse.list_scroll)
                    .vertical_scrollbar(&self.browse.list_scroll)
                    .child(div().pr_3().child(self.render_results(cx))),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .child(self.render_list_footer(compact, cx)),
            )
            .into_any_element()
    }

    fn render_list_footer(&self, compact: bool, cx: &mut Context<Self>) -> AnyElement {
        let source = h_flex()
            .id("packages-source")
            .debug_selector(|| "packages-pager".into())
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
            .debug_selector(|| "packages-footer".into())
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

    /// Fixed heading and actions surround the independently scrolling details.
    fn render_detail_pane(&self, name: &str, narrow: bool, cx: &mut Context<Self>) -> AnyElement {
        let refreshing = matches!(self.browse.details.get(name), Some(DetailState::Loading(_)));
        let target = name.to_owned();
        let refresh = Button::new("package-details-refresh")
            .ghost()
            .small()
            .icon(IconName::RotateCw)
            .tooltip(t(cx, "packages-refresh-details"))
            .accessibility_label(t(cx, "packages-refresh-details"))
            .loading(refreshing)
            .disabled(refreshing)
            .on_click(cx.listener(move |this, _, _, cx| this.load_details(target.clone(), cx)));
        let header = if narrow {
            h_flex().justify_between().child(
                Button::new("packages-back")
                    .ghost()
                    .small()
                    .icon(IconName::ChevronLeft)
                    .label(t(cx, "packages-back"))
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.browse.focus.focus(window, cx);
                        this.browse.showing_detail = false;
                        cx.notify();
                    })),
            )
        } else {
            h_flex().gap_2().items_center().child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(name.to_owned()),
            )
        };
        let header = header.child(refresh).when(!narrow, |header| {
            header.child(
                Button::new("packages-close")
                    .ghost()
                    .small()
                    .icon(IconName::X)
                    .tooltip(t(cx, "packages-close"))
                    .accessibility_label(t(cx, "packages-close"))
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.packages_focus.focus(window, cx);
                        this.browse.selected = None;
                        this.browse.showing_detail = false;
                        cx.notify();
                    })),
            )
        });
        let body = self.render_package_detail(name, narrow, cx);
        let ready = match self.browse.details.get(name) {
            Some(DetailState::Ready(d)) => Some(d),
            _ => None,
        };
        let entry = self
            .browse
            .page
            .as_ref()
            .and_then(|page| page.entries.iter().find(|entry| entry.name == name));
        let repository = ready
            .and_then(|d| d.repository.clone())
            .or_else(|| entry.and_then(|e| e.repository.clone()));
        let npm = entry.and_then(|entry| entry.npm.clone());
        package_detail(
            "packages-detail",
            narrow,
            header.into_any_element(),
            body,
            self.render_detail_actions(name, repository, npm, cx),
            &self.browse.detail_scroll,
        )
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
            if self.browse.searching.is_some() || self.browse.debounce.is_some() {
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
                package_row(
                    SharedString::from(format!("package-entry-{}", entry.name)),
                    selected,
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
                        }))
                        .into_any_element(),
                )
                .border_color(cx.theme().border)
                .when(selected, |row| row.track_focus(&self.browse.focus))
                .on_click(cx.listener(move |this, _, _, cx| this.select_package(name.clone(), cx))),
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

        view.child(self.render_detail_status(name, cx))
            .into_any_element()
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

    fn render_detail_status(&self, name: &str, cx: &mut Context<Self>) -> AnyElement {
        let configured = self.config.read(cx).preferences(cx).pi_command;
        let pi_ready = self
            .applied_pi
            .read(cx)
            .ready_for(configured.as_deref())
            .is_some();
        let installed = self.installed_package(name, cx);
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
        }
        view = view.child(
            div()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(t(cx, "packages-trust")),
        );
        if self.active().read(cx).catalog().data().is_none() {
            view = view.child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(t(
                        cx,
                        if self.active().read(cx).catalog().is_running() {
                            "settings-resource-loading"
                        } else {
                            "packages-local-unavailable"
                        },
                    )),
            );
            if let Some(error) = self.active().read(cx).catalog().problem() {
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
        if let Some(result) = self.package_results.get(name) {
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
        view.into_any_element()
    }

    fn render_detail_actions(
        &self,
        name: &str,
        repository: Option<String>,
        npm: Option<String>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let installed = self.installed_package(name, cx).cloned();
        let source = installed
            .as_ref()
            .map(|package| package.source.clone())
            .unwrap_or_else(|| catalog::source(name));
        let mutation = self.active().read(cx).mutation();
        let updating = mutation.package_running(&source, "update");
        let installing = mutation.package_running(&source, "install");
        let removing = mutation.package_running(&source, "remove");
        let disabled = !self.package_available(cx);
        let delta = installed.as_ref().is_some_and(|package| package.is_delta());
        let actions = if delta && self.in_project_scope() {
            h_flex().child(div().text_sm().child(t(cx, "packages-project-delta")))
        } else if installed.is_some() {
            let update = source.clone();
            let remove = source.clone();
            h_flex()
                .gap_2()
                .when(!self.in_project_scope(), |row| {
                    row.child(
                        Button::new("package-detail-update")
                            .small()
                            .label(t(cx, "settings-package-update"))
                            .loading(updating)
                            .disabled(disabled)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.package("update", update.clone(), cx)
                            })),
                    )
                })
                .child(
                    Button::new("package-detail-remove")
                        .danger()
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
                    .label(self.package_install_label(cx))
                    .loading(installing)
                    .disabled(disabled)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.package("install", install.clone(), cx)
                    })),
            )
        };
        let copy = source.clone();
        let actions = actions.flex_wrap().child(
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
        actions.into_any_element()
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

#[cfg(test)]
mod tests {
    use super::{FailedSearch, Problem, ResourcesView, SearchPage};
    use crate::features::settings::{AppConfig, AppLanguage, ConfigController, PiProbeController};
    use gpui_form::Form;
    use gpui_kit::{AppContext, TestAppContext};

    struct EmptyView;
    impl gpui_kit::Render for EmptyView {
        fn render(
            &mut self,
            _: &mut gpui_kit::Window,
            _: &mut gpui_kit::Context<Self>,
        ) -> impl gpui_kit::IntoElement {
            gpui_kit::div()
        }
    }

    #[gpui_kit::test]
    fn removal_results_use_registry_identity_without_an_installed_package(cx: &mut TestAppContext) {
        use gupi_resources::pi_resources::Error;
        use gupi_resources::resources::{Change, ResourceEvent};
        cx.update(|cx| {
            gpui_kit::init(cx);
            crate::app::init_capability_hosts(cx);
            app_theme::init(cx);
            gupi_settings::theme::init(cx);
            gupi_settings::i18n::apply(AppLanguage::English, cx);
        });
        let mut resources = None;
        let (_, visual) = cx.add_window_view(|window, cx| {
            let form = cx.new(|_| Form::new(AppConfig::default()));
            let config = cx.new(|cx| ConfigController::new(&form, cx));
            let probe = cx.new(|_| PiProbeController::new());
            resources = Some(cx.new(|cx| ResourcesView::new(config, probe, window, cx)));
            let view = cx.new(|_| EmptyView);
            gpui_kit::component::Root::new(view, window, cx)
        });
        let resources = resources.unwrap();
        for (source, name, result) in [
            ("npm:foo@1.2.0", "foo", Ok(())),
            (
                "npm:@scope/foo@latest",
                "@scope/foo",
                Err(Error::new("partial removal".into())),
            ),
        ] {
            visual.update(|_, cx| {
                let controller = resources.read(cx).controller.clone();
                controller.update(cx, |_, cx| {
                    cx.emit(ResourceEvent::Finished {
                        target: source.into(),
                        result: result.clone(),
                        change: Change::Package {
                            command: "pi".into(),
                            action: "remove",
                            source: source.into(),
                        },
                    })
                });
            });
            visual.run_until_parked();
            visual.update(|_, cx| {
                let view = resources.read(cx);
                assert!(view.installed_package(name, cx).is_none());
                let actual = view
                    .package_results
                    .get(name)
                    .expect("result survives missing installed entry");
                assert_eq!(
                    actual.as_ref().map_err(ToString::to_string),
                    result.as_ref().map_err(ToString::to_string)
                );
                assert!(!view.package_results.contains_key(source));
            });
        }
    }

    #[gpui_kit::test]
    fn language_changes_preserve_resource_searches_and_results(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_kit::init(cx);
            crate::app::init_capability_hosts(cx);
            app_theme::init(cx);
            gupi_settings::theme::init(cx);
            gupi_settings::i18n::apply(AppLanguage::English, cx);
        });
        let mut resources = None;
        let (_, visual) = cx.add_window_view(|window, cx| {
            let form = cx.new(|_| Form::new(AppConfig::default()));
            let config = cx.new(|cx| ConfigController::new(&form, cx));
            let probe = cx.new(|_| PiProbeController::new());
            let view = cx.new(|cx| ResourcesView::new(config, probe, window, cx));
            view.update(cx, |view, cx| {
                for (input, value) in [
                    (&view.browse.query, "pi-mcp"),
                    (&view.search, "review"),
                    (&view.prompt_search, "explain"),
                ] {
                    input.update(cx, |input, cx| input.set_value(value, window, cx));
                }
                view.browse.page = Some(super::catalog::parse_search(
                    br#"{"objects":[{"package":{"name":"pi-mcp-adapter","version":"5.2.0","keywords":["pi-package"]}}],"total":1}"#,
                    0,
                ).unwrap());
                view.browse.selected = Some("pi-mcp-adapter".into());
                view.browse.details.insert(
                    "pi-mcp-adapter".into(),
                    super::DetailState::Failed(Problem::Network),
                );
                let mut catalog = gupi_resources::pi_resources::Catalog::default();
                catalog.resources = [
                    (gupi_resources::pi_resources::Kind::Skill, "review"),
                    (gupi_resources::pi_resources::Kind::Prompt, "explain"),
                ].into_iter().map(|(kind, name)| {
                    gupi_resources::pi_resources::Resource::new(
                        kind, name.into(), "".into(), name.into(),
                    )
                }).collect();
                view.controller.update(cx, |controller, _| controller.set_catalog_for_test(catalog));
            });
            resources = Some(view);
            EmptyView
        });
        let resources = resources.unwrap();
        visual.run_until_parked();
        for language in [AppLanguage::Chinese, AppLanguage::English] {
            visual.update(|_, cx| gupi_settings::i18n::apply(language, cx));
            visual.run_until_parked();
            visual.update(|_, cx| {
                let view = resources.read(cx);
                for (input, value, key) in [
                    (&view.browse.query, "pi-mcp", "packages-search"),
                    (&view.search, "review", "settings-skill-search"),
                    (&view.prompt_search, "explain", "settings-template-search"),
                ] {
                    let input = input.read(cx);
                    assert_eq!(input.value().as_ref(), value);
                    assert_eq!(
                        input.presentation().placeholder(),
                        &gupi_settings::i18n::t(cx, key)
                    );
                }
                assert_eq!(
                    view.browse.page.as_ref().unwrap().entries[0].name,
                    "pi-mcp-adapter"
                );
                assert_eq!(view.browse.selected.as_deref(), Some("pi-mcp-adapter"));
                assert!(view.browse.details.contains_key("pi-mcp-adapter"));
                assert_eq!(view.browse.serial, 0);
                assert!(view.browse.debounce.is_none());
                assert!(view.browse.searching.is_none());
                assert_eq!(view.filtered_skills(cx)[0].name, "review");
                assert_eq!(view.filtered_prompts(cx)[0].name, "explain");
            });
        }
    }

    #[gpui_kit::test]
    fn changing_query_clears_results_and_failure_before_debounce(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_kit::init(cx);
            crate::app::init_capability_hosts(cx);
            app_theme::init(cx);
            gupi_settings::theme::init(cx);
            gupi_settings::i18n::apply(AppLanguage::English, cx);
        });
        cx.add_window_view(|window, cx| {
            let form = cx.new(|_| Form::new(AppConfig::default()));
            let config = cx.new(|cx| ConfigController::new(&form, cx));
            let probe = cx.new(|_| PiProbeController::new());
            let resources = cx.new(|cx| ResourcesView::new(config, probe, window, cx));
            resources.update(cx, |view, cx| {
                view.browse.page = Some(SearchPage::default());
                view.browse.problem = Some(FailedSearch {
                    query: "old".into(),
                    from: 20,
                    problem: Problem::Network,
                });
                view.browse.selected = Some("old-package".into());
                view.browse.showing_detail = true;
                let serial = view.browse.serial;
                view.schedule_search(cx);
                assert!(view.browse.page.is_none());
                assert!(view.browse.problem.is_none());
                assert!(view.browse.selected.is_none());
                assert!(!view.browse.showing_detail);
                assert!(view.browse.searching.is_none());
                assert!(view.browse.serial > serial);
                assert!(view.browse.debounce.is_some());
                view.browse.debounce = None;
            });
            EmptyView
        });
    }
}

#[cfg(test)]
mod layout_tests {
    use super::*;
    use crate::features::settings::layout::{Domain, PageId};
    use crate::features::settings::{
        AppConfig, AppLanguage, ConfigController, PiProbeController, SettingsView,
    };
    use core::prelude::v1::test;
    use gpui_form::Form;
    use gpui_kit::test::TestWindowExt as _;

    #[gpui_kit::test]
    fn packages_bound_scrolling_and_retain_navigation(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_kit::init(cx);
            gpui_tokio::init(cx);
            crate::app::init_capability_hosts(cx);
            app_theme::init(cx);
            gupi_settings::theme::init(cx);
            gupi_settings::i18n::apply(AppLanguage::English, cx);
        });
        let agent = tempfile::tempdir().unwrap();
        let mut settings = None;
        let (_, visual) = cx.add_window_view(|window, cx| {
            let form = cx.new(|_| Form::new(AppConfig::default()));
            let config = cx.new(|cx| ConfigController::new(&form, cx));
            let draft = cx.new(|_| PiProbeController::new());
            let applied = cx.new(|_| { let mut probe = PiProbeController::new(); probe.stop(); probe });
            let view = cx.new(|cx| SettingsView::new(form, config, draft, applied, cx.focus_handle(), window, cx));
            view.update(cx, |view, cx| {
                view.auth.update(cx, |auth, _| auth.set_agent_for_test(agent.path().to_owned()));
                view.pi_config.update(cx, |config, cx| config.set_agent_for_test(agent.path().to_owned(), cx));
                view.open_page(Domain::Pi, PageId::Packages, window, cx);
                view.resources.update(cx, |view, cx| {
                    view.controller.update(cx, |owner, _| {
                        let mut catalog = io::Catalog::default();
                        catalog.packages = (0..60).map(|ix| io::Package::new(format!("npm:package-{ix}"), agent.path().to_owned())).collect();
                        catalog.resources = (0..60).map(|ix| {
                            let mut r = Resource::new(Kind::Skill, agent.path().join(format!("skill-{ix}.md")), agent.path().to_owned(), format!("Skill {ix}"));
                            r.package = Some("npm:package-0".into()); r
                        }).collect();
                        owner.set_catalog_for_test(catalog);
                    });
                    let entries: Vec<_> = (0..20).map(|ix| serde_json::json!({"package":{"name":format!("package-{ix}"),"version":"1.0.0","description":"Example package","keywords":["pi-package"]}})).collect();
                    view.browse.page = Some(catalog::parse_search(&serde_json::to_vec(&serde_json::json!({"objects": entries,"total":40})).unwrap(),0).unwrap());
                    view.browse.details.insert("package-0".into(), DetailState::Failed(Problem::Network));
                });
            });
            settings = Some(view.clone());
            gpui_kit::component::Root::new(view, window, cx)
        });
        let settings = settings.unwrap();
        visual.simulate_resize(size(px(1500.), px(760.)));
        visual.run_until_parked();
        visual.update(|window, cx| {
            window.render_frame(cx);
            window.click(
                ElementId::NamedChild(
                    std::sync::Arc::new(ElementId::from(SharedString::from("false:npm:package-0"))),
                    "select".into(),
                ),
                cx,
            );
        });
        visual.run_until_parked();
        visual.update(|window, cx| window.render_frame(cx));
        let toolbar = visual.debug_bounds("packages-toolbar").unwrap();
        let header = visual.debug_bounds("package-detail-heading").unwrap();
        let actions = visual.debug_bounds("package-detail-actions").unwrap();
        let list = visual.debug_bounds("packages-installed").unwrap();
        assert!(list.size.height > px(100.) && list.bottom() <= px(760.));
        assert!(header.top() >= toolbar.bottom() && actions.bottom() <= px(760.));
        visual.update(|window, cx| {
            window.scroll(
                "packages-installed",
                ScrollDelta::Pixels(point(px(0.), px(-240.))),
                cx,
            );
            window.scroll(
                "installed-package-detail",
                ScrollDelta::Pixels(point(px(0.), px(-180.))),
                cx,
            );
        });
        visual.run_until_parked();
        visual.update(|window, cx| window.render_frame(cx));
        assert_eq!(visual.debug_bounds("packages-toolbar").unwrap(), toolbar);
        assert_eq!(
            visual.debug_bounds("package-detail-heading").unwrap(),
            header
        );
        assert_eq!(
            visual.debug_bounds("package-detail-actions").unwrap(),
            actions
        );
        visual.update(|window, cx| {
            window.within("packages-view").click(1usize, cx);
        });
        visual.run_until_parked();
        visual.update(|window, cx| window.render_frame(cx));
        visual.update(|window, cx| window.click("package-entry-package-0", cx));
        visual.run_until_parked();
        visual.update(|window, cx| window.render_frame(cx));
        let footer = visual.debug_bounds("packages-footer").unwrap();
        visual.update(|window, cx| {
            window.scroll(
                "packages-results",
                ScrollDelta::Pixels(point(px(0.), px(-160.))),
                cx,
            )
        });
        visual.run_until_parked();
        visual.update(|window, cx| window.render_frame(cx));
        assert_eq!(visual.debug_bounds("packages-footer").unwrap(), footer);
        let offset = visual.update(|_, cx| {
            settings
                .read(cx)
                .resources
                .read(cx)
                .browse
                .list_scroll
                .offset()
        });
        assert!(offset.y < px(0.));
        visual.simulate_resize(size(px(800.), px(760.)));
        visual.run_until_parked();
        visual.update(|window, cx| {
            window.render_frame(cx);
            window.click("packages-back", cx);
        });
        visual.run_until_parked();
        visual.update(|window, cx| {
            window.render_frame(cx);
            let view = settings.read(cx).resources.read(cx);
            assert_eq!(view.browse.selected.as_deref(), Some("package-0"));
            assert_eq!(view.browse.list_scroll.offset(), offset);
            assert!(view.browse.focus.is_focused(window));
            window.within("packages-view").click(0usize, cx);
        });
        visual.run_until_parked();
        visual.update(|window, cx| {
            window.render_frame(cx);
            window.click("installed-back", cx);
        });
        visual.run_until_parked();
        visual.simulate_resize(size(px(1500.), px(760.)));
        visual.run_until_parked();
        visual.update(|window, cx| {
            window.render_frame(cx);
            assert!(window.try_find("installed-close").is_some());
        });
    }
}
