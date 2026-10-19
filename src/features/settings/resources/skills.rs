use super::*;
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::component::collapsible::Collapsible;
use gpui_kit::component::group_box::GroupBox;
use gpui_kit::component::group_box::GroupBoxVariants;
use gpui_kit::component::menu::ContextMenuExt;
use gpui_kit::component::menu::DropdownMenu;
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::component::setting::SettingItem;
use gpui_kit::component::tag::Tag;
use gpui_kit::component::text::TextView;
use gpui_kit::component::tooltip::Tooltip;

// These handles belong to the same expanded preview as its asynchronous content.
pub(super) struct Preview {
    pub content: refresh::Operation<String, io::Error, Task<()>>,
    pub focus: FocusHandle,
    pub scroll: ScrollHandle,
}

gpui_kit::actions!(skill_preview, [PageUp, PageDown, Start, End]);
struct PreviewKeys;
impl Global for PreviewKeys {}
pub(super) fn init(cx: &mut App) {
    if !cx.has_global::<PreviewKeys>() {
        cx.bind_keys([
            KeyBinding::new("pageup", PageUp, Some("SkillPreview")),
            KeyBinding::new("pagedown", PageDown, Some("SkillPreview")),
            KeyBinding::new("home", Start, Some("SkillPreview")),
            KeyBinding::new("end", End, Some("SkillPreview")),
        ]);
        cx.set_global(PreviewKeys);
    }
}
fn scroll_preview(handle: &ScrollHandle, pages: f32, window: &mut Window) {
    let mut offset = handle.offset();
    offset.y =
        (offset.y - handle.bounds().size.height * pages).clamp(-handle.max_offset().y, px(0.));
    handle.set_offset(offset);
    window.refresh();
}

impl ResourcesView {
    pub(super) fn filtered_skills(&self, cx: &App) -> Vec<Resource> {
        self.skills_in(self.active(), cx)
    }

    fn skills_in(&self, controller: &Entity<ResourceController>, cx: &App) -> Vec<Resource> {
        let row = self.row(controller == &self.controller);
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
                    self.resource_source(resource, row, cx)
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
        let row = view.row(inherited);
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
                        .update(cx, |this, cx| this.render_skill(&resource, row, cx))
                        .unwrap_or_else(|_| div().into_any_element())
                })
                .keywords(keywords)
            })
            .collect()
    }

    fn render_skill(&self, resource: &Resource, row: Row, cx: &Context<Self>) -> AnyElement {
        let mut displayed = resource.clone();
        displayed.enabled = self.resource_enabled(resource, row, cx);
        let resource = &displayed;

        let busy = self.busy(cx);
        let relation = self.relation(resource, row, cx);
        let path = resource.path.clone();
        let key = self.preview_key(resource, row, cx);
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
                    .dropdown_menu_with_anchor(
                        Anchor::TopRight,
                        self.resource_menu(resource, row, cx),
                    ),
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
                    .child(self.render_source(resource, row, cx))
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
                    // The row catalog determines the destination settings file.
                    .when(self.can_toggle(resource, row, cx), |header| {
                        header.child(
                            Switch::new(ElementId::NamedChild(id.clone(), "enabled".into()))
                                .checked(resource.enabled)
                                .accessibility_label(resource.name.clone())
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
                        .line_clamp(3)
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
            if !resource.description.is_empty() {
                content = content.child(
                    div()
                        .id(ElementId::NamedChild(id.clone(), "description".into()))
                        .role(Role::Paragraph)
                        .aria_label(resource.description.clone())
                        .test_support()
                        .text_sm()
                        .child(resource.description.clone()),
                );
            }
            if preview.content.is_running() {
                content = content.child(gpui_kit::component::spinner::Spinner::new().small());
            }
            if let Some(error) = preview.content.problem() {
                content = content.child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().danger)
                        .child(error.to_string()),
                );
            }
            if let Some(text) = preview.content.data() {
                content = content.child(TextView::markdown(
                    ElementId::NamedChild(id.clone(), "markdown".into()),
                    preview_body(text).to_owned(),
                ));
            }
            let scroll = preview.scroll.clone();
            let up = scroll.clone();
            let down = scroll.clone();
            let start = scroll.clone();
            let end = scroll.clone();
            card = card.content(
                content
                    .id(ElementId::NamedChild(id.clone(), "preview-scroll".into()))
                    .test_support()
                    .h(rems(20.))
                    .w_full()
                    .border_1()
                    .rounded_md()
                    .border_color(cx.theme().border)
                    .focus(|style| style.border_color(cx.theme().ring))
                    .track_focus(&preview.focus)
                    .key_context("SkillPreview")
                    .on_action(move |_: &PageUp, window, _| scroll_preview(&up, -1., window))
                    .on_action(move |_: &PageDown, window, _| scroll_preview(&down, 1., window))
                    .on_action(move |_: &Start, window, _| {
                        scroll_preview(&start, f32::NEG_INFINITY, window)
                    })
                    .on_action(move |_: &End, window, _| {
                        scroll_preview(&end, f32::INFINITY, window)
                    })
                    .overflow_y_scroll()
                    .track_scroll(&scroll)
                    .vertical_scrollbar(&scroll),
            );
        }
        div()
            .id(ElementId::NamedChild(id.clone(), "card".into()))
            .child(
                GroupBox::new()
                    .outline()
                    .content_style(StyleRefinement::default().p_3().bg(cx.theme().background))
                    .child(card),
            )
            .context_menu(self.resource_menu(resource, row, cx))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::Row;
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

    struct PreviewFixture {
        owner: gpui_kit::Entity<super::ResourcesView>,
        resource: super::Resource,
    }
    impl gpui_kit::Render for PreviewFixture {
        fn render(
            &mut self,
            _: &mut gpui_kit::Window,
            cx: &mut gpui_kit::Context<Self>,
        ) -> impl gpui_kit::IntoElement {
            self.owner.update(cx, |view, cx| {
                view.render_skill(&self.resource, Row::Global, cx)
            })
        }
    }

    #[gpui_kit::test]
    async fn expanded_skill_keeps_full_description_actions_and_keyboard_scroll(
        cx: &mut TestAppContext,
    ) {
        use gpui_kit::ElementId;
        use gpui_kit::test::TestWindowExt as _;
        cx.executor().allow_parking();
        cx.update(|cx| {
            gpui_kit::init(cx);
            gpui_tokio::init(cx);
            crate::app::init_capability_hosts(cx);
            app_theme::init(cx);
            gupi_settings::theme::init(cx);
            gupi_settings::i18n::apply(AppLanguage::English, cx);
        });
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("SKILL.md");
        let description = "Complete description stays available beyond the summary. ".repeat(35);
        let text = format!(
            "---\nname: review\ndescription: {description}\n---\n{}",
            "Body paragraph.\n\n".repeat(60)
        );
        std::fs::write(&path, &text).unwrap();
        let mut resource = super::Resource::new(
            super::Kind::Skill,
            path.clone(),
            dir.path().to_owned(),
            "review".into(),
        );
        resource.description = description.clone();
        resource.editable = true;
        let mut owner = None;
        let (_, cx) = cx.add_window_view(|window, cx| {
            let form = cx.new(|_| Form::new(AppConfig::default()));
            let config = cx.new(|cx| ConfigController::new(&form, cx));
            let probe = cx.new(|_| PiProbeController::new());
            let resources = cx.new(|cx| super::ResourcesView::new(config, probe, window, cx));
            owner = Some(resources.clone());
            let fixture = cx.new(|_| PreviewFixture {
                owner: resources,
                resource: resource.clone(),
            });
            Root::new(fixture, window, cx)
        });
        let owner = owner.unwrap();
        let key = owner.update(cx, |view, cx| {
            let key = view.preview_key(&resource, Row::Global, cx);
            view.toggle_preview(key.clone(), cx);
            key
        });
        cx.condition(&owner, |view, _| !view.previews[&key].content.is_running())
            .await;
        let id = std::sync::Arc::new(key.element_id());
        cx.update(|window, cx| {
            window.render_frame(cx);
            assert_eq!(
                window
                    .find(ElementId::NamedChild(id.clone(), "description".into()))
                    .label(),
                Some(description.as_str())
            );
            let preview = owner.read(cx).previews.get(&key).unwrap();
            let scroll = preview.scroll.clone();
            let focus = preview.focus.clone();
            window.focus(&focus, cx);
            window.render_frame(cx);
            assert!(scroll.max_offset().y > gpui_kit::px(0.));
            window.press("pagedown", cx);
            assert!(
                scroll.offset().y < gpui_kit::px(0.),
                "PageDown must scroll the focused preview"
            );
            window.press("home", cx);
            assert_eq!(scroll.offset().y, gpui_kit::px(0.));
            let actions = window
                .find(ElementId::NamedChild(id.clone(), "actions".into()))
                .bounds();
            let content = window
                .find(ElementId::NamedChild(id.clone(), "preview-scroll".into()))
                .bounds();
            assert!(
                actions.bottom() <= content.top(),
                "actions stay outside and above scrolling content"
            );
            assert!(
                window
                    .find(ElementId::NamedChild(id.clone(), "actions".into()))
                    .visible()
            );
            window.press("tab", cx);
            assert!(
                !owner.read(cx).previews[&key]
                    .focus
                    .contains_focused(window, cx),
                "Tab can leave the region"
            );
        });
        assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
        std::fs::remove_file(&path).unwrap();
        owner.update(cx, |view, cx| view.load_preview(key.clone(), cx));
        cx.condition(&owner, |view, _| !view.previews[&key].content.is_running())
            .await;
        cx.update(|window, cx| {
            window.render_frame(cx);
            assert!(owner.read(cx).previews[&key].content.problem().is_some());
            assert_eq!(
                window
                    .find(ElementId::NamedChild(id.clone(), "description".into()))
                    .label(),
                Some(description.as_str())
            );
        });
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
            config.set_agent_for_test(agent.path().to_owned(), cx);
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

        let skill = resources.read_with(cx, |view, cx| view.filtered_skills(cx)[0].clone());
        resources.update(cx, |view, cx| {
            assert!(view.can_toggle(&skill, Row::Own, cx));
            assert!(!view.can_toggle(&skill, Row::Inherited, cx));
            view.change(
                gupi_resources::resources::Change::Toggle(skill.clone(), false),
                cx,
            );
        });
        cx.condition(&resources, |view, cx| {
            !view.active().read(cx).busy() && !view.filtered_skills(cx)[0].enabled
        })
        .await;
        let settings_path = project.path().join(".pi/settings.json");
        let settings: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&settings_path).unwrap()).unwrap();
        assert_eq!(
            settings["skills"],
            serde_json::json!(["-skills/review/SKILL.md"])
        );
        assert!(!agent.path().join("settings.json").exists());
        // Saving a raw path refreshes the already-selected project catalog.
        let prompt = trust::canonical(project.path()).join("extra.md");
        std::fs::write(&prompt, "Extra prompt").unwrap();
        cx.run_until_parked();
        cx.update(|window, cx| {
            pi_config.update(cx, |config, cx| {
                config.append_prompt_path_for_test("../extra.md".into(), cx);
                config.save(
                    Some(crate::features::settings::pi_config::Page::Tools),
                    None,
                    window,
                    cx,
                );
            })
        });
        cx.condition(&resources, |view, cx| {
            view.active()
                .read(cx)
                .catalog()
                .data()
                .is_some_and(|catalog| catalog.resources.iter().any(|r| r.path == prompt))
        })
        .await;

        pi_config.update(cx, |config, cx| config.set_scope(Scope::Global, cx));
        cx.run_until_parked();
        resources.read_with(cx, |view, _| assert!(!view.in_project_scope()));
    }
    #[gpui_kit::test]
    async fn identical_package_resources_keep_catalog_previews_and_navigation_separate(
        cx: &mut TestAppContext,
    ) {
        // The project scan runs on a blocking thread outside the test executor.
        cx.executor().allow_parking();
        let agent = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();
        let package = tempfile::tempdir().unwrap();
        let source = package.path().to_string_lossy().into_owned();
        let skill = package.path().join("skills/review/SKILL.md");
        let prompt = package.path().join("prompts/review.md");
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
            serde_json::to_vec(&serde_json::json!({"packages": [source]})).unwrap(),
        )
        .unwrap();
        std::fs::write(
            agent.path().join("settings.json"),
            serde_json::to_vec(&serde_json::json!({"packages": [source]})).unwrap(),
        )
        .unwrap();
        std::fs::write(package.path().join("package.json"), "{}").unwrap();
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
            config.set_agent_for_test(agent.path().to_owned(), cx);
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
                assert_eq!(
                    own, inherited,
                    "the same local package yields identical values"
                );
                assert_ne!(view.owner_of(Row::Own), view.owner_of(Row::Inherited));
                let own_key = view.preview_key(own, Row::Own, cx);
                let inherited_key = view.preview_key(inherited, Row::Inherited, cx);
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
                view.expanded_packages.clear();
                view.reveal_package(source.clone(), Row::Inherited, cx);
                assert!(view.expanded_packages.contains(&format!("true:{source}")));
                assert!(!view.expanded_packages.contains(&format!("false:{source}")));
                view.expanded_packages.clear();
                view.reveal_package(source.clone(), Row::Own, cx);
                assert!(view.expanded_packages.contains(&format!("false:{source}")));
                assert!(!view.expanded_packages.contains(&format!("true:{source}")));
            }
        });
        // A project delta changes the inherited display only after trust, and
        // never mutates the global catalog or invents a same-name relation.
        std::fs::write(
            project.path().join(".pi/settings.json"),
            serde_json::to_vec(
                &serde_json::json!({"packages":[{"source":source,"autoload":false,
                "skills":["!skills/**"],"prompts":["!prompts/**"]}]}),
            )
            .unwrap(),
        )
        .unwrap();
        let delta = gupi_resources::pi_resources::scan_project(project.path()).unwrap();
        resources.update(cx, |view, cx| {
            view.project = Some(cx.new(|_| {
                let mut controller = gupi_resources::resources::ResourceController::for_target(
                    gupi_resources::resources::Target::Project(project.path().to_owned()),
                );
                controller.set_catalog_for_test(delta);
                controller
            }));
            for resource in &global.resources {
                assert!(view.resource_enabled(resource, Row::Inherited, cx));
            }
        });
        trust::trust(agent.path(), project.path()).unwrap();
        pi_config.update(cx, |config, cx| config.reload(cx));
        cx.condition(&pi_config, |config, _| config.trusted() == Some(true))
            .await;
        resources.update(cx, |view, cx| {
            // Install a fresh controller for the updated catalog: Load does
            // not replace an already loaded operation in this test fixture.
            let delta = gupi_resources::pi_resources::scan_project(project.path()).unwrap();
            view.project = Some(cx.new(|_| {
                let mut controller = gupi_resources::resources::ResourceController::for_target(
                    gupi_resources::resources::Target::Project(project.path().to_owned()),
                );
                controller.set_catalog_for_test(delta);
                controller
            }));
            view.controller.update(cx, |controller, _| {
                controller.set_catalog_for_test(global.clone())
            });
            for resource in &global.resources {
                assert!(!view.resource_enabled(resource, Row::Inherited, cx));
                assert!(view.relation(resource, Row::Inherited, cx).is_none());
                assert!(view.resource_enabled(resource, Row::Global, cx));
                assert!(resource.enabled);
            }
        });
    }
}
