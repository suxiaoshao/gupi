use super::*;
use gpui_kit::component::Icon;
use gpui_kit::component::Selectable;
use gpui_kit::component::collapsible::Collapsible;
use gpui_kit::component::group_box::GroupBox;
use gpui_kit::component::group_box::GroupBoxVariants;
use gpui_kit::component::menu::ContextMenuExt;
use gpui_kit::component::menu::DropdownMenu;
use gpui_kit::component::setting::SettingItem;
use gpui_kit::component::tag::Tag;
use gpui_kit::component::text::TextView;
use gpui_kit::component::tooltip::Tooltip;

impl ResourcesView {
    pub(super) fn filtered_prompts(&self, cx: &App) -> Vec<Resource> {
        self.prompts_in(self.active(), cx)
    }

    fn prompts_in(&self, controller: &Entity<ResourceController>, cx: &App) -> Vec<Resource> {
        let query = self.prompt_search.read(cx).value().trim().to_lowercase();
        controller
            .read(cx)
            .catalog()
            .data()
            .into_iter()
            .flat_map(|catalog| &catalog.resources)
            .filter(|resource| resource.kind == Kind::Prompt)
            .filter(|resource| {
                format!(
                    "/{} {} {}",
                    resource.name,
                    resource.description,
                    self.resource_source(resource, cx)
                )
                .to_lowercase()
                .contains(&query)
            })
            .cloned()
            .collect()
    }

    /// Rows for the current scope, or with `inherited`, the global rows shown
    /// read-only beneath a project's own.
    pub(crate) fn prompt_items(
        owner: &Entity<Self>,
        inherited: bool,
        cx: &App,
    ) -> Vec<SettingItem> {
        let view = owner.read(cx);
        let resources = if inherited {
            view.prompts_in(&view.controller, cx)
        } else {
            view.filtered_prompts(cx)
        };
        resources
            .into_iter()
            .map(|resource| {
                let owner = owner.downgrade();
                let keywords = vec![
                    resource.name.clone(),
                    resource.description.clone(),
                    "prompt template 模板 提示词".into(),
                ];
                SettingItem::render(move |_, _, cx| {
                    owner
                        .update(cx, |this, cx| this.render_prompt(&resource, cx))
                        .unwrap_or_else(|_| div().into_any_element())
                })
                .keywords(keywords)
            })
            .collect()
    }

    pub(crate) fn system_prompt_items(owner: &Entity<Self>, cx: &App) -> Vec<SettingItem> {
        [
            (
                "SYSTEM.md",
                "settings-system-replace",
                "settings-system-replace-help",
            ),
            (
                "APPEND_SYSTEM.md",
                "settings-system-append",
                "settings-system-append-help",
            ),
        ]
        .into_iter()
        .map(|(file, title, help)| {
            let owner = owner.downgrade();
            SettingItem::render(move |_, _, cx| {
                owner
                    .update(cx, |this, cx| {
                        this.render_system_prompt(file, title, help, cx)
                    })
                    .unwrap_or_else(|_| div().into_any_element())
            })
            .keywords(vec![
                t(cx, title),
                t(cx, help),
                file.into(),
                "system prompt 系统提示词".into(),
            ])
        })
        .collect()
    }

    fn render_system_prompt(
        &self,
        file: &'static str,
        title: &str,
        help: &str,
        cx: &Context<Self>,
    ) -> AnyElement {
        let owner = self.active().clone();
        let Some(catalog) = owner.read(cx).catalog().data() else {
            return div().into_any_element();
        };
        let busy = self.busy(cx) || self.open.is_running();
        let path = catalog.root.join(file);
        let tooltip = path.display().to_string();
        let project = match owner.read(cx).target() {
            Target::Project(cwd) => Some(cwd.clone()),
            Target::Global => None,
        };
        // In a project, a missing file means Pi falls back to the global one.
        // An existing file, even an empty one, replaces it.
        let present = project.is_some() && path.exists();
        let open = Open {
            path: path.clone(),
            kind: Kind::Prompt,
            editable: true,
            create: false,
            owner: owner.clone(),
        };
        let fallback = (project.clone(), path.clone(), owner.clone());
        GroupBox::new()
            .outline()
            .content_style(StyleRefinement::default().p_3().bg(cx.theme().background))
            .child(
                h_flex()
                    .gap_3()
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .gap_1()
                            .child(
                                div()
                                    .id(file)
                                    .child(t(cx, title))
                                    .tooltip(move |window, cx| {
                                        Tooltip::new(tooltip.clone()).build(window, cx)
                                    }),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(t(cx, help)),
                            ),
                    )
                    .when(project.is_some(), |row| {
                        row.child(Tag::secondary().small().outline().child(t(
                            cx,
                            if present {
                                "settings-system-project-file"
                            } else {
                                "settings-system-uses-global"
                            },
                        )))
                    })
                    .when(present, |row| {
                        row.child(
                            Button::new(SharedString::from(format!("{file}-use-global")))
                                .small()
                                .ghost()
                                .label(t(cx, "settings-system-use-global"))
                                .disabled(busy)
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    let (Some(cwd), path, owner) = fallback.clone() else {
                                        return;
                                    };
                                    this.confirm_use_global(cwd, path, owner, window, cx)
                                })),
                        )
                    })
                    .child(
                        Button::new(SharedString::from(format!("{file}-edit")))
                            .small()
                            .icon(IconName::SquarePen)
                            .tooltip(t(cx, "settings-resource-edit"))
                            .accessibility_label(t(cx, "settings-resource-edit"))
                            .disabled(busy)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                let mut open = open.clone();
                                // A link out of the project's `.pi` opens read-only.
                                if let Some(cwd) = &project {
                                    open.editable = io::project_owns(cwd, &open.path);
                                }
                                this.request_open(open, window, cx)
                            })),
                    ),
            )
            .into_any_element()
    }

    /// Removes the project's SYSTEM or APPEND_SYSTEM file so Pi falls back to
    /// the global one. Saving an empty file would not.
    fn confirm_use_global(
        &mut self,
        cwd: PathBuf,
        path: PathBuf,
        owner: Entity<ResourceController>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let mut resource = Resource::new(
            Kind::Prompt,
            path.clone(),
            io::project_root(&cwd),
            name.clone(),
        );
        resource.editable = io::project_owns(&cwd, &path);
        let view = cx.entity().downgrade();
        window.open_dialog(cx, move |dialog, _, cx| {
            let view = view.clone();
            let resource = resource.clone();
            let owner = owner.clone();
            let mut args = fluent_bundle::FluentArgs::new();
            args.set("file", name.clone());
            dialog
                .title(gupi_settings::i18n::t_with_args(
                    cx,
                    "settings-system-use-global-title",
                    &args,
                ))
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(resource.path.display().to_string()),
                )
                .child(t(cx, "settings-system-use-global-body"))
                .on_ok(move |_, _, cx| {
                    view.update(cx, |this, cx| {
                        if this.busy(cx) {
                            return false;
                        }
                        this.change_in(&owner, Change::Delete(resource.clone()), cx);
                        true
                    })
                    .unwrap_or(false)
                })
        });
    }

    fn render_prompt(&self, resource: &Resource, cx: &Context<Self>) -> AnyElement {
        let busy = self.busy(cx);
        let row = self.row(resource, cx);
        let relation = self.relation(resource, cx);
        let id = |part: &'static str| {
            ElementId::NamedChild(
                std::sync::Arc::new(ElementId::from(SharedString::from(
                    resource.path.to_string_lossy().into_owned(),
                ))),
                part.into(),
            )
        };
        let preview = self.previews.get(&resource.path);
        let expanded = preview.is_some();
        let toggle_path = resource.path.clone();
        let toggle = resource.clone();
        let preview_label = t(
            cx,
            if expanded {
                "settings-template-collapse"
            } else {
                "settings-resource-view"
            },
        );
        let actions = h_flex()
            .flex_shrink_0()
            .gap_1()
            .debug_selector(|| "template-actions".into())
            .child(
                Button::new(id("preview"))
                    .ghost()
                    .small()
                    .icon(IconName::Eye)
                    .selected(expanded)
                    .tooltip(preview_label.clone())
                    .accessibility_label(preview_label)
                    .debug_selector(|| "template-preview-toggle".into())
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if this.previews.remove(&toggle_path).is_none() {
                            this.load_preview(toggle_path.clone(), cx);
                        }
                        cx.notify();
                    })),
            )
            // Enabling writes settings, which project scope leaves alone.
            .when(row == Row::Global, |actions| {
                actions.child(
                    Switch::new(id("enabled"))
                        .checked(resource.enabled)
                        .disabled(busy)
                        .on_click(cx.listener(move |this, active, _, cx| {
                            this.change(Change::Toggle(toggle.clone(), *active), cx)
                        })),
                )
            })
            .child(
                Button::new(id("actions"))
                    .ghost()
                    .small()
                    .icon(IconName::Ellipsis)
                    .tooltip(t(cx, "settings-resource-actions"))
                    .accessibility_label(t(cx, "settings-resource-actions"))
                    .debug_selector(|| "template-actions-menu".into())
                    .dropdown_menu_with_anchor(Anchor::TopRight, self.resource_menu(resource, cx)),
            );
        let source = self.resource_source(resource, cx);
        let path_help = resource.path.display().to_string();
        let mut card = Collapsible::new()
            .open(expanded)
            .gap_2()
            .child(
                h_flex()
                    .flex_wrap()
                    .gap_2()
                    .debug_selector(|| "template-card-header".into())
                    .child(
                        h_flex()
                            .flex_1()
                            .min_w(px(160.))
                            .gap_2()
                            .child(Icon::new(IconName::FileText).small())
                            .child(
                                div()
                                    .id(id("name"))
                                    .min_w_0()
                                    .truncate()
                                    .child(format!("/{}", resource.name))
                                    .tooltip(move |window, cx| {
                                        Tooltip::new(path_help.clone()).build(window, cx)
                                    }),
                            ),
                    )
                    .child(actions),
            )
            .when(!resource.description.is_empty(), |card| {
                card.child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(resource.description.clone()),
                )
            })
            .child(
                h_flex()
                    .gap_2()
                    .child(if resource.package.is_some() {
                        self.render_source(resource, cx)
                    } else {
                        Tag::secondary()
                            .small()
                            .outline()
                            .child(
                                h_flex()
                                    .gap_1()
                                    .child(
                                        Icon::new(if resource.editable {
                                            IconName::UserRound
                                        } else {
                                            IconName::LockKeyhole
                                        })
                                        .xsmall(),
                                    )
                                    .child(source),
                            )
                            .into_any_element()
                    })
                    .when(row != Row::Global && !resource.enabled, |tags| {
                        tags.child(
                            Tag::secondary()
                                .small()
                                .outline()
                                .child(t(cx, "settings-resource-disabled")),
                        )
                    })
                    .when(!resource.editable || row == Row::Inherited, |tags| {
                        tags.child(
                            Tag::secondary()
                                .small()
                                .outline()
                                .child(t(cx, "settings-resource-readonly-badge")),
                        )
                    })
                    .when_some(relation, |tags, relation| {
                        tags.child(Tag::secondary().small().outline().child(relation))
                    }),
            );
        if let Some(preview) = preview {
            let mut content = v_flex()
                .min_w_0()
                .gap_2()
                .pt_3()
                .border_t_1()
                .border_color(cx.theme().border)
                .debug_selector(|| "template-preview-content".into());
            if preview.is_running() {
                content = content.child(gpui_kit::component::spinner::Spinner::new().small());
            }
            if let Some(error) = preview.problem() {
                content = content.child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().danger)
                        .child(error.to_string()),
                );
            }
            if let Some(text) = preview.data() {
                content = content.child(TextView::markdown(
                    id("markdown"),
                    preview_body(text).to_owned(),
                ));
            }
            card = card.content(content);
        }
        div()
            .id(id("card"))
            .child(
                GroupBox::new()
                    .outline()
                    .content_style(StyleRefinement::default().p_3().bg(cx.theme().background))
                    .child(card),
            )
            .context_menu(self.resource_menu(resource, cx))
            .into_any_element()
    }
}
