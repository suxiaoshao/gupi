use super::*;
use gpui_kit::base::StyledExt as _;
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::component::Icon;
use gpui_kit::component::Selectable;
use gpui_kit::component::group_box::GroupBox;
use gpui_kit::component::group_box::GroupBoxVariants;
use gpui_kit::component::menu::DropdownMenu;
use gpui_kit::component::resizable::{ResizableState, h_resizable, resizable_panel};
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::component::tooltip::Tooltip;

const RESOURCE_KINDS: [(Kind, &str, IconName); 4] = [
    (
        Kind::Extension,
        "settings-package-kind-extensions",
        IconName::Puzzle,
    ),
    (
        Kind::Skill,
        "settings-package-kind-skills",
        IconName::BookOpen,
    ),
    (
        Kind::Theme,
        "settings-package-kind-themes",
        IconName::Palette,
    ),
    (
        Kind::Prompt,
        "settings-package-kind-prompts",
        IconName::FileText,
    ),
];

/// Transient installed-package navigation, reset when the scope owner changes.
pub(super) struct Installed {
    selected: Option<(bool, String)>,
    showing_detail: bool,
    list_scroll: ScrollHandle,
    detail_scroll: ScrollHandle,
    split: Entity<ResizableState>,
    focus: FocusHandle,
}
impl Installed {
    #[cfg(test)]
    pub(super) fn is_selected(&self, inherited: bool, source: &str) -> bool {
        self.selected
            .as_ref()
            .is_some_and(|(owner, selected)| *owner == inherited && selected == source)
    }

    pub fn new(cx: &mut App) -> Self {
        Self {
            selected: None,
            showing_detail: false,
            list_scroll: ScrollHandle::new(),
            detail_scroll: ScrollHandle::new(),
            split: cx.new(|_| ResizableState::default()),
            focus: cx.focus_handle().tab_stop(true),
        }
    }
    pub fn select(&mut self, inherited: bool, source: String) {
        if self.selected.as_ref() != Some(&(inherited, source.clone())) {
            self.detail_scroll.set_offset(Point::default());
        }
        self.selected = Some((inherited, source));
        self.showing_detail = true;
    }
}

pub(super) fn package_row(id: impl Into<ElementId>, selected: bool, content: AnyElement) -> Button {
    Button::new(id)
        .ghost()
        .selected(selected)
        .w_full()
        .h_auto()
        .py_2()
        .justify_start()
        .border_b_1()
        .child(content)
}

pub(super) fn package_panes(
    id: &'static str,
    width: Pixels,
    split: &Entity<ResizableState>,
    list: AnyElement,
    detail: Option<AnyElement>,
    showing_detail: bool,
) -> AnyElement {
    match detail {
        Some(detail) if width >= px(640.) => h_resizable(id)
            .with_state(split)
            .child(
                resizable_panel()
                    .size(px((width.as_f32() * 0.45).max(280.)))
                    .size_range(px(260.)..px((width.as_f32() - 320.).max(260.)))
                    .child(list),
            )
            .child(resizable_panel().child(detail))
            .into_any_element(),
        Some(detail) if showing_detail => detail,
        _ => list,
    }
}

pub(super) fn package_detail(
    id: &'static str,
    narrow: bool,
    header: AnyElement,
    body: AnyElement,
    footer: AnyElement,
    scroll: &ScrollHandle,
) -> AnyElement {
    v_flex()
        .id(format!("{id}-pane"))
        .size_full()
        .min_h_0()
        .min_w_0()
        .gap_3()
        .when(!narrow, |pane| pane.pl_4())
        .child(
            div()
                .flex_shrink_0()
                .debug_selector(|| "package-detail-heading".into())
                .child(header),
        )
        .child(
            div()
                .id(id)
                .test_support()
                .debug_selector(|| "package-detail-body".into())
                .relative()
                .flex_1()
                .min_h_0()
                .min_w_0()
                .overflow_y_scroll()
                .track_scroll(scroll)
                .vertical_scrollbar(scroll)
                .child(div().pr_3().child(body)),
        )
        .child(
            div()
                .flex_shrink_0()
                .debug_selector(|| "package-detail-actions".into())
                .child(footer),
        )
        .into_any_element()
}

impl ResourcesView {
    /// The Packages body; `width` is its own measured width.
    pub(super) fn render_packages(&mut self, width: Pixels, cx: &mut Context<Self>) -> AnyElement {
        let status = self
            .has_status(Kind::Extension, cx)
            .then(|| self.render_page(Kind::Extension, Section::Overview, width, cx));
        let body = if self.browsing {
            self.render_browse(width, cx)
        } else {
            self.render_installed(width, cx)
        };
        v_flex()
            .size_full()
            .min_h_0()
            .min_w_0()
            .gap_3()
            .children(status.map(|status| div().flex_shrink_0().child(status)))
            .child(div().flex_1().min_h_0().min_w_0().child(body))
            .into_any_element()
    }

    pub(super) fn reconcile_package_selection(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let missing =
            self.installed
                .selected
                .as_ref()
                .is_some_and(|(inherited, source)| {
                    let owner = if *inherited {
                        &self.controller
                    } else {
                        self.active()
                    };
                    owner.read(cx).catalog().data().is_some_and(|catalog| {
                        !catalog.packages.iter().any(|p| &p.source == source)
                    })
                });
        if missing {
            self.installed.selected = None;
            self.installed.showing_detail = false;
            self.packages_focus.focus(window, cx);
        }
    }

    pub(in crate::features::settings) fn render_packages_tabs(
        &self,
        cx: &Context<Self>,
    ) -> AnyElement {
        use gpui_kit::component::tab::Tab;
        use gpui_kit::component::tab::TabBar;
        TabBar::new("packages-view")
            .segmented()
            .small()
            .selected_index(usize::from(self.browsing))
            .child(Tab::new().label(t(cx, "packages-installed-tab")))
            .child(Tab::new().label(t(cx, "packages-browse-tab")))
            .on_click(cx.listener(|this, index: &usize, _, cx| {
                this.browsing = *index == 1;
                cx.notify();
            }))
            .into_any_element()
    }

    pub(in crate::features::settings) fn render_packages_toolbar(
        &self,
        cx: &Context<Self>,
    ) -> AnyElement {
        h_flex()
            .debug_selector(|| "packages-toolbar".into())
            .w_full()
            .min_w_0()
            .flex_wrap()
            .gap_2()
            .child(
                div()
                    .track_focus(&self.packages_focus)
                    .child(self.render_packages_tabs(cx)),
            )
            .child(div().flex_1().min_w_0().when(self.browsing, |body| {
                body.child(
                    self.browse
                        .search_input()
                        .prefix(Icon::new(IconName::Search)),
                )
            }))
            .when(!self.browsing, |bar| {
                bar.child(
                    Button::new("package-add")
                        .small()
                        .icon(IconName::Plus)
                        .label(t(cx, "settings-package-install-source"))
                        .disabled(!self.package_available(cx) || self.installing)
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.installing = true;
                            this.source.focus_handle(cx).focus(window, cx);
                            cx.notify();
                        })),
                )
            })
            .child(self.render_header(Kind::Extension, cx))
            .into_any_element()
    }

    pub(super) fn package_install_label(&self, cx: &App) -> String {
        if let Some(project) = &self.project
            && let Target::Project(cwd) = project.read(cx).target()
        {
            let mut args = fluent_bundle::FluentArgs::new();
            args.set(
                "folder",
                cwd.file_name()
                    .unwrap_or(cwd.as_os_str())
                    .to_string_lossy()
                    .into_owned(),
            );
            return gupi_settings::i18n::t_with_args(cx, "packages-install-project", &args);
        }
        t(cx, "packages-install")
    }

    fn render_installed(&self, width: Pixels, cx: &Context<Self>) -> AnyElement {
        let controller = self.active().read(cx);
        let Some(catalog) = controller.catalog().data() else {
            return h_flex()
                .gap_2()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .when(controller.catalog().is_running(), |body| {
                    body.child(gpui_kit::component::spinner::Spinner::new().small())
                })
                .child(t(
                    cx,
                    if controller.catalog().is_running() {
                        "settings-resource-loading"
                    } else {
                        "packages-local-unavailable"
                    },
                ))
                .into_any_element();
        };
        let busy = controller.busy() || self.config.read(cx).busy(cx);
        let configured = self.config.read(cx).preferences(cx).pi_command;
        let pi_ready = self
            .applied_pi
            .read(cx)
            .ready_for(configured.as_deref())
            .is_some();
        let mut toolbar = v_flex().flex_shrink_0().gap_3();
        if !pi_ready {
            toolbar = toolbar.child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(t(cx, "settings-resource-pi-required")),
            );
        }
        if self.installing {
            toolbar = toolbar
                .child(
                    gpui_kit::component::form::field()
                        .label(t(cx, "settings-package-source"))
                        .description(t(cx, "settings-package-source-help"))
                        .child(Input::new(&self.source).disabled(!self.package_available(cx))),
                )
                .child(
                    h_flex()
                        .gap_2()
                        .child(
                            Button::new("package-install")
                                .primary()
                                .label(self.package_install_label(cx))
                                .disabled(
                                    !self.package_available(cx)
                                        || self.source.read(cx).value().trim().is_empty(),
                                )
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.package(
                                        "install",
                                        this.source.read(cx).value().trim().into(),
                                        cx,
                                    );
                                })),
                        )
                        .child(
                            Button::new("package-cancel")
                                .ghost()
                                .label(t(cx, "action-cancel"))
                                .disabled(busy)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.installing = false;
                                    cx.notify();
                                })),
                        ),
                );
        }
        let mut view = v_flex().gap_3();
        if self.in_project_scope() {
            view = view.child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(cx.theme().muted_foreground)
                    .child(t(cx, "settings-source-project")),
            );
        }
        if catalog.packages.is_empty()
            && !(self.in_project_scope()
                && catalog.resources.iter().any(|r| {
                    r.package.is_none() && matches!(r.kind, Kind::Extension | Kind::Theme)
                }))
        {
            view = view.child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(t(
                        cx,
                        if self.in_project_scope() {
                            "settings-project-package-empty"
                        } else {
                            "settings-package-empty"
                        },
                    )),
            );
        }
        for package in &catalog.packages {
            view = view.child(self.render_package(package, catalog, false, cx));
        }
        view = view.child(self.render_independent_resources(catalog, self.row(false), busy, cx));
        if self.in_project_scope()
            && let Some(global) = self.controller.read(cx).catalog().data()
        {
            view = view.child(
                div()
                    .mt_2()
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(cx.theme().muted_foreground)
                    .child(t(cx, "settings-resource-group-inherited")),
            );
            for package in &global.packages {
                view = view.child(self.render_package(package, global, true, cx));
            }
            view = view.child(self.render_independent_resources(global, Row::Inherited, busy, cx));
        }

        let list = div()
            .id("packages-installed")
            .test_support()
            .debug_selector(|| "packages-installed".into())
            .relative()
            .size_full()
            .min_h_0()
            .min_w_0()
            .overflow_y_scroll()
            .track_scroll(&self.installed.list_scroll)
            .vertical_scrollbar(&self.installed.list_scroll)
            .child(div().pr_3().child(view))
            .into_any_element();
        let detail = self
            .installed
            .selected
            .as_ref()
            .and_then(|(inherited, source)| {
                let owner = if *inherited {
                    &self.controller
                } else {
                    self.active()
                };
                let catalog = owner.read(cx).catalog().data()?;
                let package = catalog.packages.iter().find(|p| &p.source == source)?;
                Some(self.render_installed_detail(
                    package,
                    catalog,
                    busy,
                    *inherited,
                    width < px(640.),
                    cx,
                ))
            });
        v_flex()
            .size_full()
            .min_h_0()
            .min_w_0()
            .gap_3()
            .child(toolbar)
            .child(div().flex_1().min_h_0().min_w_0().child(package_panes(
                "packages-installed-split",
                width,
                &self.installed.split,
                list,
                detail,
                self.installed.showing_detail,
            )))
            .into_any_element()
    }

    fn render_package(
        &self,
        package: &io::Package,
        catalog: &io::Catalog,
        inherited: bool,
        cx: &Context<Self>,
    ) -> AnyElement {
        let source = package.source.clone();
        let key = format!("{inherited}:{source}");
        let selected = self.installed.selected.as_ref() == Some(&(inherited, source.clone()));
        let id = |part: &str| {
            ElementId::NamedChild(
                std::sync::Arc::new(ElementId::from(SharedString::from(key.clone()))),
                part.to_owned().into(),
            )
        };

        let name = package_name(package);
        let tooltip = source.clone();
        let mut heading = v_flex()
            .w_full()
            .min_w_0()
            .flex_1()
            .gap_1()
            .text_left()
            .child(
                h_flex()
                    .gap_2()
                    .min_w_0()
                    .child(
                        div()
                            .id(id("name"))
                            .min_w_0()
                            .flex_1()
                            .truncate()
                            .font_weight(FontWeight::MEDIUM)
                            .child(name)
                            .tooltip(move |window, cx| {
                                Tooltip::new(tooltip.clone()).build(window, cx)
                            }),
                    )
                    .children(package.version.clone().map(|version| {
                        div()
                            .flex_shrink_0()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(version)
                    })),
            );
        heading = heading.child(
            div()
                .id(id("source"))
                .min_w_0()
                .text_sm()
                .truncate()
                .text_color(cx.theme().muted_foreground)
                .child(match io_counts(catalog, &source, cx) {
                    Some(counts) => format!("{} · {source}", counts.join(" · ")),
                    None => source.clone(),
                })
                .tooltip({
                    let source = source.clone();
                    move |window, cx| Tooltip::new(source.clone()).build(window, cx)
                }),
        );
        if self.in_project_scope() {
            let other = if inherited {
                self.active()
            } else {
                &self.controller
            };
            let related = other.read(cx).catalog().data().and_then(|other| {
                let identity = io::package_identity(&catalog.root, &source)?;
                other.packages.iter().find(|p| {
                    io::package_identity(&other.root, &p.source).as_ref() == Some(&identity)
                })
            });
            let label = if package.is_delta() {
                "packages-project-delta"
            } else if inherited && related.is_some_and(|p| p.is_delta()) {
                "packages-project-filtered"
            } else if inherited && related.is_some() {
                "packages-project-replaced"
            } else if inherited {
                "settings-resource-group-inherited"
            } else if related.is_some() {
                "packages-project-replaces"
            } else {
                "settings-source-project"
            };
            if !inherited || related.is_some() {
                heading = heading
                    .child(h_flex().child(Tag::secondary().small().outline().child(t(cx, label))));
            }
            if !inherited
                && self.pi_config.as_ref().and_then(|c| c.read(cx).trusted()) == Some(false)
            {
                heading = heading.child(
                    h_flex().child(
                        Tag::secondary()
                            .small()
                            .outline()
                            .child(t(cx, "settings-resource-not-loaded")),
                    ),
                );
            }
        }
        if !package.is_delta() && !package.path.exists() {
            heading = heading.child(
                h_flex().child(
                    Tag::danger()
                        .small()
                        .outline()
                        .child(t(cx, "packages-files-missing")),
                ),
            );
        }
        package_row(id("select"), selected, heading.into_any_element())
            .border_color(cx.theme().border)
            .when(selected, |row| row.track_focus(&self.installed.focus))
            .debug_selector({
                let source = source.clone();
                move || format!("package-select-{inherited}-{source}")
            })
            .on_click(cx.listener(move |this, _, _, cx| {
                this.installed.select(inherited, source.clone());
                cx.notify();
            }))
            .into_any_element()
    }

    fn render_installed_detail(
        &self,
        package: &io::Package,
        catalog: &io::Catalog,
        busy: bool,
        inherited: bool,
        narrow: bool,
        cx: &Context<Self>,
    ) -> AnyElement {
        let source = package.source.clone();
        let reveal = package.path.clone();
        let update = source.clone();
        let remove = source.clone();
        let id = |part: &str| SharedString::from(format!("installed-detail-{part}"));
        let package_disabled = !self.package_available(cx);
        let mutable = !inherited && !(self.in_project_scope() && package.is_delta());
        let update_visible = !self.in_project_scope();
        let pi_config = self.pi_config.clone();
        let edit_global = self.pi_config.clone();
        let pending = self
            .active()
            .read(cx)
            .mutation()
            .package_running(&source, "update");
        let copy = source.clone();
        let menu =
            move |menu: gpui_kit::component::menu::PopupMenu,
                  _: &mut Window,
                  cx: &mut Context<gpui_kit::component::menu::PopupMenu>| {
                let reveal = reveal.clone();
                let copy = copy.clone();
                menu.item(
                    PopupMenuItem::new(t(cx, "settings-resource-location"))
                        .icon(IconName::FolderOpen)
                        .on_click(move |_, _, cx| cx.reveal_path(&reveal)),
                )
                .item(
                    PopupMenuItem::new(t(cx, "packages-copy-source"))
                        .icon(IconName::Copy)
                        .on_click(move |_, _, cx| {
                            cx.write_to_clipboard(ClipboardItem::new_string(copy.clone()))
                        }),
                )
                .when(inherited, |menu| {
                    let pi_config = pi_config.clone();
                    menu.item(
                        PopupMenuItem::new(t(cx, "settings-resource-edit-global"))
                            .icon(IconName::SquarePen)
                            .on_click(move |_, window, cx| {
                                if let Some(config) = &pi_config {
                                    super::super::pi_config::request_scope(
                                        config,
                                        Scope::Global,
                                        window,
                                        cx,
                                    );
                                }
                            }),
                    )
                })
            };
        let actions = h_flex()
            .gap_2()
            .flex_wrap()
            .when(inherited, |row| {
                row.child(
                    Button::new(id("global"))
                        .small()
                        .label(t(cx, "settings-resource-edit-global"))
                        .on_click(move |_, window, cx| {
                            if let Some(config) = &edit_global {
                                super::super::pi_config::request_scope(
                                    config,
                                    Scope::Global,
                                    window,
                                    cx,
                                );
                            }
                        }),
                )
            })
            .when(update_visible && !inherited, |row| {
                row.child(
                    Button::new(id("update"))
                        .small()
                        .label(t(cx, "settings-package-update"))
                        .loading(pending)
                        .disabled(package_disabled)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.package("update", update.clone(), cx)
                        })),
                )
            })
            .when(mutable, |row| {
                row.child(
                    Button::new(id("remove"))
                        .danger()
                        .small()
                        .label(t(cx, "settings-package-remove-ellipsis"))
                        .disabled(package_disabled)
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.confirm_remove(None, remove.clone(), window, cx)
                        })),
                )
            })
            .child(
                Button::new(id("more"))
                    .ghost()
                    .small()
                    .icon(IconName::Ellipsis)
                    .tooltip(t(cx, "settings-resource-actions"))
                    .accessibility_label(t(cx, "settings-resource-actions"))
                    .dropdown_menu_with_anchor(Anchor::TopRight, menu),
            );
        let header = h_flex()
            .gap_2()
            .min_w_0()
            .when(narrow, |header| {
                header.child(
                    Button::new("installed-back")
                        .ghost()
                        .small()
                        .icon(IconName::ChevronLeft)
                        .label(t(cx, "packages-back"))
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.installed.showing_detail = false;
                            this.installed.focus.focus(window, cx);
                            cx.notify();
                        })),
                )
            })
            .when(!narrow, |header| {
                header.child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_lg()
                        .font_semibold()
                        .child(package_name(package)),
                )
            })
            .child(self.render_header(Kind::Extension, cx))
            .when(!narrow, |header| {
                header.child(
                    Button::new("installed-close")
                        .ghost()
                        .small()
                        .icon(IconName::X)
                        .tooltip(t(cx, "packages-close"))
                        .accessibility_label(t(cx, "packages-close"))
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.packages_focus.focus(window, cx);
                            this.installed.selected = None;
                            this.installed.showing_detail = false;
                            cx.notify();
                        })),
                )
            });
        let mut contents = v_flex()
            .gap_3()
            .pt_3()
            .border_t_1()
            .border_color(cx.theme().border)
            .when(narrow, |body| {
                body.child(div().text_lg().font_semibold().child(package_name(package)))
            })
            .children(package.version.clone().map(|version| {
                let mut args = fluent_bundle::FluentArgs::new();
                args.set("version", version);
                div().text_sm().child(gupi_settings::i18n::t_with_args(
                    cx,
                    "packages-installed-version",
                    &args,
                ))
            }))
            .child(div().text_sm().child(source.clone()));
        let mut has_resources = false;
        for (kind, label, icon) in RESOURCE_KINDS {
            let resources: Vec<_> = catalog
                .resources
                .iter()
                .filter(|r| r.package.as_deref() == Some(source.as_str()) && r.kind == kind)
                .collect();
            if resources.is_empty() {
                continue;
            }
            has_resources = true;
            contents = contents.child(
                v_flex()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(t(cx, label)),
                    )
                    .children(resources.into_iter().map(|resource| {
                        self.render_package_resource(
                            resource,
                            self.row(inherited),
                            icon,
                            false,
                            busy,
                            cx,
                        )
                    })),
            );
        }
        if !has_resources {
            contents = contents.child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(t(cx, "settings-package-resources-empty")),
            );
        }
        package_detail(
            "installed-package-detail",
            narrow,
            header.into_any_element(),
            contents.into_any_element(),
            actions.into_any_element(),
            &self.installed.detail_scroll,
        )
    }

    fn render_independent_resources(
        &self,
        catalog: &io::Catalog,
        row: Row,
        busy: bool,
        cx: &Context<Self>,
    ) -> AnyElement {
        let mut groups = v_flex().gap_3();
        for (kind, icon, title) in [
            (
                Kind::Extension,
                IconName::Puzzle,
                "settings-extension-heading",
            ),
            (Kind::Theme, IconName::Palette, "settings-theme-heading"),
        ] {
            if kind == Kind::Theme && row != Row::Own {
                continue;
            }
            let resources: Vec<_> = catalog
                .resources
                .iter()
                .filter(|r| r.kind == kind && r.package.is_none())
                .collect();
            if resources.is_empty() {
                continue;
            }
            groups = groups.child(
                div()
                    .mt_2()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(t(cx, title)),
            );
            for resource in resources {
                groups = groups.child(
                    GroupBox::new()
                        .outline()
                        .content_style(StyleRefinement::default().p_3().bg(cx.theme().background))
                        .child(self.render_package_resource(resource, row, icon, true, busy, cx)),
                );
            }
        }
        groups.into_any_element()
    }

    fn render_package_resource(
        &self,
        resource: &Resource,
        row: Row,
        icon: IconName,
        independent: bool,
        busy: bool,
        cx: &Context<Self>,
    ) -> AnyElement {
        let mut displayed = resource.clone();
        displayed.enabled = self.resource_enabled(resource, row, cx);
        let resource = &displayed;

        let path = resource.path.clone();
        let id = |part: &str| {
            ElementId::NamedChild(
                std::sync::Arc::new(ElementId::from(SharedString::from(format!(
                    "{:?}:{}:{}",
                    row,
                    resource.base.display(),
                    resource.path.display()
                )))),
                part.to_owned().into(),
            )
        };
        let toggle = resource.clone();
        let copy_path = path.clone();
        h_flex()
            .gap_2()
            .debug_selector({
                let path = path.clone();
                move || format!("package-resource-{}", path.display())
            })
            .child(Icon::new(icon).small())
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap_1()
                    .child(
                        div()
                            .id(id("name"))
                            .truncate()
                            .child(resource.name.clone())
                            .tooltip({
                                let path = path.clone();
                                move |window, cx| {
                                    Tooltip::new(path.to_string_lossy().into_owned())
                                        .build(window, cx)
                                }
                            }),
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .flex_wrap()
                            .when(
                                row == Row::Own
                                    && self.pi_config.as_ref().and_then(|c| c.read(cx).trusted())
                                        == Some(false),
                                |row| {
                                    row.child(
                                        Tag::secondary()
                                            .small()
                                            .outline()
                                            .child(t(cx, "settings-resource-not-loaded")),
                                    )
                                },
                            )
                            .when(self.in_project_scope() && !resource.enabled, |row| {
                                row.child(
                                    Tag::secondary()
                                        .small()
                                        .outline()
                                        .child(t(cx, "settings-resource-disabled")),
                                )
                            })
                            .when(self.in_project_scope() && independent, |tags| {
                                tags.child(
                                    Tag::secondary()
                                        .small()
                                        .outline()
                                        .child(self.resource_source(resource, row, cx)),
                                )
                                .child(
                                    Tag::secondary()
                                        .small()
                                        .outline()
                                        .child(t(cx, "settings-resource-readonly-badge")),
                                )
                            }),
                    )
                    .when(independent, |row| {
                        row.child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .truncate()
                                .child(path.to_string_lossy().into_owned()),
                        )
                    }),
            )
            .when(independent, |row| {
                row.child(
                    Button::new(id("path"))
                        .ghost()
                        .small()
                        .icon(IconName::FolderOpen)
                        .tooltip(t(cx, "settings-resource-location"))
                        .accessibility_label(t(cx, "settings-resource-location"))
                        .on_click(move |_, _, cx| cx.reveal_path(&path)),
                )
            })
            .when(self.in_project_scope() && independent, |row| {
                row.child(
                    Button::new(id("copy"))
                        .ghost()
                        .small()
                        .icon(IconName::Copy)
                        .tooltip(t(cx, "files-copy-path"))
                        .accessibility_label(t(cx, "files-copy-path"))
                        .on_click(move |_, _, cx| {
                            cx.write_to_clipboard(ClipboardItem::new_string(
                                copy_path.display().to_string(),
                            ))
                        }),
                )
            })
            .when(self.can_toggle(resource, row, cx), |row| {
                row.child(
                    Switch::new(id("enabled"))
                        .checked(resource.enabled)
                        .accessibility_label(resource.name.clone())
                        .disabled(busy)
                        .on_click(cx.listener(move |this, enabled, _, cx| {
                            this.change(Change::Toggle(toggle.clone(), *enabled), cx);
                        })),
                )
            })
            .into_any_element()
    }
}

pub(super) fn package_name(package: &io::Package) -> String {
    if !package.source.starts_with("npm:") {
        return package
            .path
            .file_name()
            .filter(|name| !name.is_empty())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| package.source.clone());
    }
    let name = package
        .source
        .strip_prefix("npm:")
        .unwrap_or(&package.source);
    package
        .version
        .as_ref()
        .filter(|_| package.source.starts_with("npm:"))
        .and_then(|version| name.strip_suffix(&format!("@{version}")))
        .unwrap_or(name)
        .to_owned()
}
