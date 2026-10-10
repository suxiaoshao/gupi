use super::*;
use gpui_kit::component::Icon;
use gpui_kit::component::collapsible::Collapsible;
use gpui_kit::component::group_box::GroupBox;
use gpui_kit::component::group_box::GroupBoxVariants;
use gpui_kit::component::menu::DropdownMenu;
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

/// Where the Packages body is mounted. A page-filling slot bounds its height,
/// so the lists scroll inside it; a settings group-list item sizes to its
/// content and leaves scrolling to the settings page.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Mount {
    #[allow(dead_code)]
    Bounded,
    Content,
}

/// A region that owns its scrolling in a bounded body and keeps its natural
/// height otherwise.
pub(super) fn region(id: &'static str, scroll: &ScrollHandle, mount: Mount) -> Stateful<Div> {
    let region = div().id(id).min_w_0();
    match mount {
        Mount::Bounded => region
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(scroll),
        Mount::Content => region,
    }
}

impl ResourcesView {
    /// The Packages body; `width` is its own measured width.
    pub(super) fn render_packages(
        &mut self,
        width: Pixels,
        mount: Mount,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if self.browsing {
            self.render_browse(width, mount, cx)
        } else {
            self.render_installed(mount, cx)
        }
    }

    pub(super) fn render_packages_tabs(&self, cx: &Context<Self>) -> AnyElement {
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

    fn render_installed(&self, mount: Mount, cx: &Context<Self>) -> AnyElement {
        let controller = self.active().read(cx);
        let Some(catalog) = controller.catalog().data() else {
            return div().into_any_element();
        };
        let busy = controller.busy() || self.config.read(cx).busy(cx);
        let configured = self.config.read(cx).preferences(cx).pi_command;
        let pi_ready = self
            .applied_pi
            .read(cx)
            .ready_for(configured.as_deref())
            .is_some();
        let mut toolbar = v_flex().flex_shrink_0().gap_3().child(
            h_flex()
                .justify_between()
                .gap_2()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(cx.theme().muted_foreground)
                        .debug_selector(|| "package-heading".into())
                        .child(t(cx, "settings-package-heading")),
                )
                .child(
                    Button::new("package-add")
                        .small()
                        .icon(IconName::Plus)
                        .label(t(cx, "settings-package-install-source"))
                        .disabled(!self.package_available(cx) || self.installing)
                        .debug_selector(|| "package-add".into())
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.installing = true;
                            this.source.focus_handle(cx).focus(window, cx);
                            cx.notify();
                        })),
                ),
        );
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
            view = view.child(self.render_package(package, catalog, busy, false, cx));
        }
        view = view.child(self.render_independent_resources(catalog, busy, cx));
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
                view = view.child(self.render_package(package, global, busy, true, cx));
            }
            view = view.child(self.render_independent_resources(global, busy, cx));
        }

        // Content mounts use the Settings page scroller; bounded mounts own scrolling.
        v_flex()
            .size_full()
            .min_h_0()
            .gap_3()
            .child(toolbar)
            .child(region("packages-installed", &self.installed_scroll, mount).child(view))
            .into_any_element()
    }

    fn render_package(
        &self,
        package: &io::Package,
        catalog: &io::Catalog,
        busy: bool,
        inherited: bool,
        cx: &Context<Self>,
    ) -> AnyElement {
        let source = package.source.clone();
        let key = format!("{inherited}:{source}");
        let expanded = self.expanded_packages.contains(&key);
        let id = |part: &str| {
            ElementId::NamedChild(
                std::sync::Arc::new(ElementId::from(SharedString::from(key.clone()))),
                part.to_owned().into(),
            )
        };
        let toggle = key.clone();
        let reveal = package.path.clone();
        let update = source.clone();
        let remove = source.clone();
        let name = package_name(package);
        let tooltip = source.clone();
        let mut heading = v_flex().min_w_0().flex_1().gap_1().child(
            h_flex()
                .gap_2()
                .min_w_0()
                .child(
                    div()
                        .id(id("name"))
                        .min_w_0()
                        .truncate()
                        .font_weight(FontWeight::MEDIUM)
                        .child(name)
                        .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx)),
                )
                .children(package.version.clone().map(|version| {
                    div()
                        .flex_shrink_0()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(version)
                })),
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
        if let Some(counts) = io_counts(catalog, &source, cx) {
            heading = heading.child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(counts.join(" · ")),
            );
        }
        let expand = Button::new(id("expand"))
            .ghost()
            .small()
            .icon(if expanded {
                IconName::ChevronUp
            } else {
                IconName::ChevronDown
            })
            .tooltip(t(
                cx,
                if expanded {
                    "settings-package-collapse"
                } else {
                    "settings-package-expand"
                },
            ))
            .accessibility_label(t(
                cx,
                if expanded {
                    "settings-package-collapse"
                } else {
                    "settings-package-expand"
                },
            ))
            .debug_selector({
                let source = source.clone();
                move || format!("package-expand-{source}")
            })
            .on_click(cx.listener(move |this, _, _, cx| {
                if !this.expanded_packages.remove(&toggle) {
                    this.expanded_packages.insert(toggle.clone());
                }
                cx.notify();
            }));
        let package_disabled = !self.package_available(cx);
        let mutable = !inherited && !(self.in_project_scope() && package.is_delta());
        let update_visible = !self.in_project_scope();
        let pi_config = self.pi_config.clone();
        let pending = self
            .active()
            .read(cx)
            .mutation()
            .package_running(&source, "update");
        let owner = cx.entity().downgrade();
        let copy = source.clone();
        let menu =
            move |menu: gpui_kit::component::menu::PopupMenu,
                  _: &mut Window,
                  cx: &mut Context<gpui_kit::component::menu::PopupMenu>| {
                let reveal = reveal.clone();
                let copy = copy.clone();
                let owner = owner.clone();
                let remove = remove.clone();
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
                .when(mutable, |menu| {
                    menu.separator().item(
                        PopupMenuItem::new(t(cx, "settings-package-remove-ellipsis"))
                            .icon(IconName::Trash)
                            .disabled(package_disabled)
                            .on_click(move |_, window, cx| {
                                owner
                                    .update(cx, |this, cx| {
                                        this.confirm_remove(None, remove.clone(), window, cx)
                                    })
                                    .ok();
                            }),
                    )
                })
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
        let mut card = Collapsible::new().open(expanded).gap_3().child(
            h_flex().gap_2().child(heading).child(
                h_flex()
                    .flex_shrink_0()
                    .gap_1()
                    .when(update_visible, |row| {
                        row.child(
                            Button::new(id("update"))
                                .small()
                                .label(t(cx, "settings-package-update"))
                                .loading(pending)
                                .disabled(!self.package_available(cx))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.package("update", update.clone(), cx);
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
                    )
                    .child(expand),
            ),
        );
        if expanded {
            let mut contents = v_flex()
                .gap_3()
                .pt_3()
                .border_t_1()
                .border_color(cx.theme().border);
            for (kind, label, icon) in RESOURCE_KINDS {
                if kind == Kind::Theme
                    && !catalog
                        .resources
                        .iter()
                        .any(|r| r.kind == kind && self.in_project(r, cx))
                {
                    continue;
                }
                let resources: Vec<_> = catalog
                    .resources
                    .iter()
                    .filter(|r| r.package.as_deref() == Some(source.as_str()) && r.kind == kind)
                    .collect();
                if resources.is_empty() {
                    continue;
                }
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
                            self.render_package_resource(resource, icon, false, busy, cx)
                        })),
                );
            }
            card = card.content(contents);
        }
        GroupBox::new()
            .outline()
            .content_style(StyleRefinement::default().p_3().bg(cx.theme().background))
            .child(card)
            .into_any_element()
    }

    fn render_independent_resources(
        &self,
        catalog: &io::Catalog,
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
            if kind == Kind::Theme
                && !self
                    .project
                    .as_ref()
                    .and_then(|c| c.read(cx).catalog().data())
                    .is_some_and(|active| active.root == catalog.root)
            {
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
                        .child(self.render_package_resource(resource, icon, true, busy, cx)),
                );
            }
        }
        groups.into_any_element()
    }

    fn render_package_resource(
        &self,
        resource: &Resource,
        icon: IconName,
        independent: bool,
        busy: bool,
        cx: &Context<Self>,
    ) -> AnyElement {
        let path = resource.path.clone();
        let id = |part: &str| {
            ElementId::NamedChild(
                std::sync::Arc::new(ElementId::from(SharedString::from(format!(
                    "{}:{}",
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
                                self.in_project(resource, cx)
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
                            .when(self.in_project_scope() && independent, |row| {
                                row.child(
                                    Tag::secondary()
                                        .small()
                                        .outline()
                                        .child(self.resource_source(resource, cx)),
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
            .when(!self.in_project_scope(), |row| {
                row.child(
                    Switch::new(id("enabled"))
                        .checked(resource.enabled)
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
