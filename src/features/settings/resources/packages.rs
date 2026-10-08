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

    fn render_installed(&self, mount: Mount, cx: &Context<Self>) -> AnyElement {
        let controller = self.controller.read(cx);
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
                        .debug_selector(|| "package-heading".into())
                        .child(t(cx, "settings-package-heading")),
                )
                .child(
                    Button::new("package-add")
                        .small()
                        .icon(IconName::Plus)
                        .label(t(cx, "settings-package-install-source"))
                        .disabled(busy || !pi_ready || self.installing)
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
                        .child(Input::new(&self.source).disabled(busy || !pi_ready)),
                )
                .child(
                    h_flex()
                        .gap_2()
                        .child(
                            Button::new("package-install")
                                .primary()
                                .label(t(cx, "settings-package-install"))
                                .disabled(
                                    busy || !pi_ready
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
        if catalog.packages.is_empty() {
            view = view.child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(t(cx, "settings-package-empty")),
            );
        }
        for package in &catalog.packages {
            view = view.child(self.render_package(package, catalog, busy, pi_ready, cx));
        }
        let independent: Vec<_> = catalog
            .resources
            .iter()
            .filter(|resource| resource.kind == Kind::Extension && resource.package.is_none())
            .collect();
        if !independent.is_empty() {
            view = view.child(div().mt_2().child(t(cx, "settings-extension-heading")));
            for resource in independent {
                view = view.child(
                    GroupBox::new()
                        .outline()
                        .content_style(StyleRefinement::default().p_3().bg(cx.theme().background))
                        .child(self.render_package_resource(
                            resource,
                            IconName::Puzzle,
                            true,
                            busy,
                            cx,
                        )),
                );
            }
        }
        // The toolbar stays put; the installed list owns its scrolling.
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
        pi_ready: bool,
        cx: &Context<Self>,
    ) -> AnyElement {
        let source = package.source.clone();
        let expanded = self.expanded_packages.contains(&source);
        let id = |part: &str| {
            ElementId::NamedChild(
                std::sync::Arc::new(ElementId::from(SharedString::from(source.clone()))),
                part.to_owned().into(),
            )
        };
        let toggle = source.clone();
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
        if !package.path.exists() {
            heading = heading.child(
                h_flex().child(
                    Tag::danger()
                        .small()
                        .outline()
                        .child(t(cx, "packages-files-missing")),
                ),
            );
        } else if let Some(pinned) = gupi_resources::catalog::pinned_version(&source) {
            let mut args = fluent_bundle::FluentArgs::new();
            args.set("version", pinned.to_owned());
            heading = heading.child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(gupi_settings::i18n::t_with_args(
                        cx,
                        "packages-pinned-installed",
                        &args,
                    )),
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
        let pending = self.pending_package.as_deref() == Some(source.as_str());
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
                .separator()
                .item(
                    PopupMenuItem::new(t(cx, "settings-package-remove-ellipsis"))
                        .icon(IconName::Trash)
                        .disabled(busy || !pi_ready)
                        .on_click(move |_, window, cx| {
                            owner
                                .update(cx, |this, cx| {
                                    this.confirm_remove(None, remove.clone(), window, cx)
                                })
                                .ok();
                        }),
                )
            };
        let mut card = Collapsible::new().open(expanded).gap_3().child(
            h_flex().gap_2().child(heading).child(
                h_flex()
                    .flex_shrink_0()
                    .gap_1()
                    .child(
                        Button::new(id("update"))
                            .small()
                            .label(t(cx, "settings-package-update"))
                            .loading(pending)
                            .disabled(busy || !pi_ready)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.package("update", update.clone(), cx);
                            })),
                    )
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
                std::sync::Arc::new(ElementId::from(SharedString::from(
                    resource.path.to_string_lossy().into_owned(),
                ))),
                part.to_owned().into(),
            )
        };
        let toggle = resource.clone();
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
            .child(
                Switch::new(id("enabled"))
                    .checked(resource.enabled)
                    .disabled(busy)
                    .on_click(cx.listener(move |this, enabled, _, cx| {
                        this.change(Change::Toggle(toggle.clone(), *enabled), cx);
                    })),
            )
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
