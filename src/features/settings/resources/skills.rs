use super::*;
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
    pub(super) fn filtered_skills(&self, cx: &App) -> Vec<Resource> {
        self.skills_in(self.active(), cx)
    }

    fn skills_in(&self, controller: &Entity<ResourceController>, cx: &App) -> Vec<Resource> {
        let query = self.search.read(cx).value().trim().to_lowercase();
        controller
            .read(cx)
            .catalog()
            .data()
            .into_iter()
            .flat_map(|catalog| &catalog.resources)
            .filter(|resource| resource.kind == Kind::Skill)
            .filter(|resource| {
                format!(
                    "{} {} {} {}",
                    resource.name,
                    resource.description,
                    resource.path.display(),
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
    pub(crate) fn skill_items(owner: &Entity<Self>, inherited: bool, cx: &App) -> Vec<SettingItem> {
        let view = owner.read(cx);
        let resources = if inherited {
            view.skills_in(&view.controller, cx)
        } else {
            view.filtered_skills(cx)
        };
        resources
            .into_iter()
            .map(|resource| {
                let owner = owner.downgrade();
                let keywords = vec![
                    resource.name.clone(),
                    resource.description.clone(),
                    resource.path.display().to_string(),
                    "Skill 技能".into(),
                ];
                SettingItem::render(move |_, _, cx| {
                    owner
                        .update(cx, |this, cx| this.render_skill(&resource, cx))
                        .unwrap_or_else(|_| div().into_any_element())
                })
                .keywords(keywords)
            })
            .collect()
    }

    fn render_skill(&self, resource: &Resource, cx: &Context<Self>) -> AnyElement {
        let busy = self.busy(cx);
        let row = self.row(resource, cx);
        let relation = self.relation(resource, cx);
        let path = resource.path.clone();
        let key = self.preview_key(resource, cx);
        let id = std::sync::Arc::new(key.element_id());
        let preview = self.previews.get(&key);
        let expanded = preview.is_some();
        let toggle = resource.clone();
        let toggle_key = key.clone();
        let toggle_label = t(
            cx,
            if expanded {
                "settings-skill-collapse"
            } else {
                "settings-resource-view"
            },
        );
        let actions = h_flex()
            .gap_1()
            .child(
                Button::new(ElementId::NamedChild(id.clone(), "preview".into()))
                    .ghost()
                    .small()
                    .icon(if expanded {
                        IconName::ChevronUp
                    } else {
                        IconName::ChevronDown
                    })
                    .label(toggle_label)
                    .debug_selector(|| "skill-preview-toggle".into())
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.toggle_preview(toggle_key.clone(), cx);
                    })),
            )
            .child(div().flex_1())
            .child(
                Button::new(ElementId::NamedChild(id.clone(), "actions".into()))
                    .ghost()
                    .small()
                    .icon(IconName::Ellipsis)
                    .tooltip(t(cx, "settings-resource-actions"))
                    .accessibility_label(t(cx, "settings-resource-actions"))
                    .debug_selector(|| "skill-actions".into())
                    .dropdown_menu_with_anchor(Anchor::TopRight, self.resource_menu(resource, cx)),
            );

        let mut card = Collapsible::new()
            .open(expanded)
            .gap_2()
            .child(
                h_flex()
                    .gap_2()
                    .child(gpui_kit::component::Icon::new(IconName::BookOpen).small())
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .truncate()
                            .child(resource.name.clone()),
                    )
                    .child(self.render_source(resource, cx))
                    .when(row != Row::Global && !resource.enabled, |header| {
                        header.child(
                            Tag::secondary()
                                .small()
                                .outline()
                                .child(t(cx, "settings-resource-disabled")),
                        )
                    })
                    .when(!resource.editable || row == Row::Inherited, |header| {
                        header.child(
                            Tag::secondary()
                                .small()
                                .outline()
                                .child(t(cx, "settings-resource-readonly-badge")),
                        )
                    })
                    .when_some(relation, |header, relation| {
                        header.child(Tag::secondary().small().outline().child(relation))
                    })
                    // Enabling writes settings, which project scope leaves alone.
                    .when(row == Row::Global, |header| {
                        header.child(
                            Switch::new(ElementId::NamedChild(id.clone(), "enabled".into()))
                                .checked(resource.enabled)
                                .disabled(busy)
                                .on_click(cx.listener(move |this, active, _, cx| {
                                    this.change(Change::Toggle(toggle.clone(), *active), cx);
                                })),
                        )
                    }),
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
                div()
                    .id(ElementId::NamedChild(id.clone(), "path".into()))
                    .min_w_0()
                    .truncate()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(path.to_string_lossy().into_owned())
                    .tooltip(move |window, cx| {
                        Tooltip::new(path.to_string_lossy().into_owned()).build(window, cx)
                    }),
            )
            .child(actions);
        if let Some(preview) = preview {
            let mut content = v_flex()
                .min_w_0()
                .gap_2()
                .debug_selector(|| "skill-preview-content".into());
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
                    ElementId::NamedChild(id.clone(), "markdown".into()),
                    preview_body(text).to_owned(),
                ));
            }
            card = card.content(content);
        }
        div()
            .id(ElementId::NamedChild(id.clone(), "card".into()))
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

#[cfg(test)]
mod tests {
    use super::preview_body;
    use crate::features::settings::AppConfig;
    use crate::features::settings::AppLanguage;
    use crate::features::settings::ConfigController;
    use crate::features::settings::PiProbeController;
    use crate::features::settings::SettingsView;
    use gpui_form::Form;
    use gpui_kit::AppContext;
    use gpui_kit::TestAppContext;
    use gpui_kit::component::Root;
    use gupi_resources::pi_settings::Scope;
    use gupi_resources::pi_settings::trust;

    #[test]
    fn preview_omits_only_complete_frontmatter() {
        assert_eq!(preview_body("---\nname: demo\n---\n\n# Body"), "# Body");
        assert_eq!(preview_body("---\r\nname: demo\r\n---\r\n# Body"), "# Body");
        assert_eq!(preview_body("# Body\n---\ntext"), "# Body\n---\ntext");
        assert_eq!(preview_body("---\nunfinished"), "---\nunfinished");
    }

    #[gpui_kit::test]
    async fn project_scope_lists_only_the_projects_own_text_resources(cx: &mut TestAppContext) {
        // The project scan runs on a blocking thread outside the test executor.
        cx.executor().allow_parking();
        let agent = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();
        let skill = project.path().join(".pi/skills/review/SKILL.md");
        std::fs::create_dir_all(skill.parent().unwrap()).unwrap();
        std::fs::write(&skill, "---\nname: review\ndescription: Reviews\n---\n").unwrap();
        cx.update(|cx| {
            gpui_kit::init(cx);
            gpui_tokio::init(cx);
            crate::app::init_capability_hosts(cx);
            app_theme::init(cx);
            gupi_settings::theme::init(cx);
            gupi_settings::i18n::apply(AppLanguage::Chinese, cx);
        });
        let mut settings = None;
        let (_, cx) = cx.add_window_view(|window, cx| {
            let form = cx.new(|_| Form::new(AppConfig::default()));
            let controller = cx.new(|cx| ConfigController::new(&form, cx));
            let draft = cx.new(|_| PiProbeController::new());
            let applied = cx.new(|_| PiProbeController::new());
            let view = cx.new(|cx| {
                SettingsView::new(
                    form,
                    controller,
                    draft,
                    applied,
                    cx.focus_handle(),
                    window,
                    cx,
                )
            });
            settings = Some(view.clone());
            Root::new(view, window, cx)
        });
        let settings = settings.unwrap();
        let (resources, pi_config) = settings.read_with(cx, |view, _| {
            (view.resources.clone(), view.pi_config.clone())
        });
        pi_config.update(cx, |config, cx| {
            // Never the user's own agent directory.
            config.set_agent_for_test(agent.path().to_owned());
            config.set_scope(Scope::Project(project.path().to_owned()), cx);
        });
        // The view is notified when the project's catalog arrives.
        cx.condition(&resources, |view, cx| !view.filtered_skills(cx).is_empty())
            .await;
        resources.read_with(cx, |view, cx| {
            assert!(view.in_project_scope());
            let skills = view.filtered_skills(cx);
            assert_eq!(skills.len(), 1);
            assert_eq!(
                skills[0].path,
                trust::canonical(project.path()).join(".pi/skills/review/SKILL.md")
            );
            assert!(skills[0].editable);
            assert!(!view.has_unsaved(cx));
        });
        assert!(
            !agent.path().join("settings.json").exists(),
            "listing a project writes nothing"
        );

        pi_config.update(cx, |config, cx| config.set_scope(Scope::Global, cx));
        cx.run_until_parked();
        resources.read_with(cx, |view, _| assert!(!view.in_project_scope()));
    }
    #[gpui_kit::test]
    async fn duplicate_file_previews_are_independent_per_catalog(cx: &mut TestAppContext) {
        // The project scan runs on a blocking thread outside the test executor.
        cx.executor().allow_parking();
        let agent = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();
        let skill = agent.path().join("skills/review/SKILL.md");
        let prompt = agent.path().join("prompts/review.md");
        std::fs::create_dir_all(skill.parent().unwrap()).unwrap();
        std::fs::create_dir_all(prompt.parent().unwrap()).unwrap();
        std::fs::write(
            &skill,
            "---\nname: review\ndescription: Reviews\n---\nSkill body",
        )
        .unwrap();
        std::fs::write(&prompt, "Prompt body").unwrap();
        std::fs::create_dir_all(project.path().join(".pi")).unwrap();
        std::fs::write(
            project.path().join(".pi/settings.json"),
            serde_json::to_vec(&serde_json::json!({"skills": [skill], "prompts": [prompt]}))
                .unwrap(),
        )
        .unwrap();
        cx.update(|cx| {
            gpui_kit::init(cx);
            gpui_tokio::init(cx);
            crate::app::init_capability_hosts(cx);
            app_theme::init(cx);
            gupi_settings::theme::init(cx);
            gupi_settings::i18n::apply(AppLanguage::Chinese, cx);
        });
        let mut settings = None;
        let (_, cx) = cx.add_window_view(|window, cx| {
            let form = cx.new(|_| Form::new(AppConfig::default()));
            let controller = cx.new(|cx| ConfigController::new(&form, cx));
            let draft = cx.new(|_| PiProbeController::new());
            let applied = cx.new(|_| PiProbeController::new());
            let view = cx.new(|cx| {
                SettingsView::new(
                    form,
                    controller,
                    draft,
                    applied,
                    cx.focus_handle(),
                    window,
                    cx,
                )
            });
            settings = Some(view.clone());
            Root::new(view, window, cx)
        });
        let settings = settings.unwrap();
        let (resources, pi_config) = settings.read_with(cx, |view, _| {
            (view.resources.clone(), view.pi_config.clone())
        });
        pi_config.update(cx, |config, cx| {
            // Never the user's own agent directory.
            config.set_agent_for_test(agent.path().to_owned());
            config.set_scope(Scope::Project(project.path().to_owned()), cx);
        });
        cx.condition(&resources, |view, cx| !view.filtered_skills(cx).is_empty())
            .await;
        let global =
            gupi_resources::pi_resources::scan(agent.path().to_owned(), None, &[]).unwrap();
        resources.update(cx, |view, cx| {
            view.controller.update(cx, |controller, _| {
                controller.set_catalog_for_test(global.clone())
            });
            let project = view.active().read(cx).catalog().data().unwrap().clone();
            assert_eq!(project.resources.len(), 2);
            for own in &project.resources {
                let inherited = global
                    .resources
                    .iter()
                    .find(|r| r.path == own.path)
                    .unwrap();
                let own_key = view.preview_key(own, cx);
                let inherited_key = view.preview_key(inherited, cx);
                assert_ne!(own_key, inherited_key);
                assert_ne!(own_key.element_id(), inherited_key.element_id());
                view.toggle_preview(own_key.clone(), cx);
                assert!(view.previews.contains_key(&own_key));
                assert!(!view.previews.contains_key(&inherited_key));
                view.toggle_preview(inherited_key.clone(), cx);
                view.toggle_preview(own_key.clone(), cx);
                assert!(!view.previews.contains_key(&own_key));
                assert!(view.previews.contains_key(&inherited_key));
                view.toggle_preview(inherited_key, cx);
            }
        });
    }
}
