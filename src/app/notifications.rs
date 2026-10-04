//! One delivery owner for main and temporary conversations, independent of windows.
use gpui_kit::AnyWindowHandle;
use gpui_kit::App;
use gpui_kit::AppContext;
use gpui_kit::Context;
use gpui_kit::Entity;
use gpui_kit::EntityId;
use gpui_kit::Global;
use gpui_kit::SharedString;
use gpui_kit::Subscription;
use gpui_kit::SystemNotification;
use gpui_kit::WeakEntity;
use gpui_kit::Window;
use gpui_kit::WindowId;
use gpui_kit::component::WindowExt;
use gpui_kit::component::notification::Notification;
#[cfg(any(test, target_os = "macos", target_os = "windows"))]
use gupi_conversation::conversation::Activity;
use gupi_conversation::conversation::ConversationEvent;
use gupi_conversation::conversation::ConversationState;
use gupi_conversation::notifications::Kind;
use gupi_conversation::notifications::Notice;
use gupi_conversation::notifications::Severity;
use gupi_settings::i18n::t;
use gupi_settings::notifications::Preferences;
use std::collections::HashMap;

// macOS GPUI active_window() returns NSApplication.mainWindow even while the
// application is backgrounded. Use actual activation state for delivery/privacy.
fn foreground_window(cx: &mut App) -> Option<AnyWindowHandle> {
    cx.windows().into_iter().find(|handle| {
        handle
            .update(cx, |_, window, _| window.is_window_active())
            .unwrap_or(false)
    })
}
struct Notifications(Entity<Delivery>);
impl Global for Notifications {}
struct Source {
    state: WeakEntity<ConversationState>,
    _subscription: Subscription,
}
struct Presentation {
    state: WeakEntity<ConversationState>,
    visible: bool,
}
#[derive(Clone)]
pub(crate) struct Target {
    state: WeakEntity<ConversationState>,
    key: String,
    binding: u64,
    request: Option<String>,
}
impl Target {
    fn valid(&self, cx: &App) -> bool {
        self.state.upgrade().is_some_and(|owner| {
            owner.read(cx).sessions().get(&self.key).is_some_and(|s| {
                s.binding() == self.binding
                    && self.request.as_ref().is_none_or(|id| {
                        s.pending_ui().iter().any(|p| {
                            p.request.id == *id
                                && p.deadline.is_none_or(|d| d > std::time::Instant::now())
                        })
                    })
            })
        })
    }
    pub fn open(&self, cx: &mut App) {
        if !self.valid(cx) {
            return;
        }
        let Some(owner) = self.state.upgrade() else {
            return;
        };
        let temporary = owner.read(cx).is_temporary();
        let key = self.key.clone();
        // A notification is navigation, never a new request or automatic reply.
        owner.update(cx, |state, cx| state.select_existing(&key, cx));
        cx.defer(move |cx| {
            if temporary {
                super::temporary::show(cx);
            } else {
                super::show(Some(false), cx);
            }
        });
    }
}
struct Delivered {
    target: Target,
    window: Option<AnyWindowHandle>,
}
#[cfg(any(target_os = "macos", target_os = "windows"))]
#[derive(PartialEq, Eq)]
struct TrayRow {
    owner: EntityId,
    key: String,
    title: String,
    activity: Activity,
    unread: bool,
    binding: u64,
}
struct Delivery {
    sources: HashMap<EntityId, Source>,
    presentations: HashMap<WindowId, Presentation>,
    delivered: HashMap<SharedString, Delivered>,
    preferences: Preferences,
    serial: u64,
    badge: usize,
    // Retaining the completed task also prevents repeated authorization attempts.
    #[cfg(all(target_os = "macos", not(test)))]
    badge_authorization: Option<gpui_kit::Task<()>>,
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    tray: Vec<TrayRow>,
}
pub fn init(cx: &mut App) {
    #[cfg(not(test))]
    if let Err(error) = platform_ext::app::set_badge_count(0) {
        tracing::warn!(%error, "clear application badge failed");
    }
    let entity = cx.new(|_| Delivery {
        sources: HashMap::new(),
        presentations: HashMap::new(),
        delivered: HashMap::new(),
        preferences: Preferences::default(),
        serial: 0,
        badge: 0,
        #[cfg(all(target_os = "macos", not(test)))]
        badge_authorization: None,
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        tray: Vec::new(),
    });
    cx.set_global(Notifications(entity.clone()));
    cx.on_system_notification_response(move |response, cx| {
        let target = entity.update(cx, |delivery, _| delivery.delivered.remove(&response.tag));
        cx.dismiss_system_notification(&response.tag);
        if let Some(target) = target {
            target.target.open(cx);
        }
    });
}
pub fn refresh_labels(cx: &mut App) {
    if let Some(manager) = cx.try_global::<Notifications>().map(|g| g.0.clone()) {
        cx.defer(move |cx| {
            manager.update(cx, |s, cx| {
                #[cfg(any(target_os = "macos", target_os = "windows"))]
                s.tray.clear();
                s.refresh(cx);
            })
        });
    }
}
pub fn configure(preferences: Preferences, cx: &mut App) {
    if let Some(manager) = cx.try_global::<Notifications>().map(|g| g.0.clone()) {
        manager.update(cx, |s, _| s.preferences = preferences);
    }
}
pub fn attach(owner: &Entity<ConversationState>, cx: &mut App) {
    let Some(manager) = cx.try_global::<Notifications>().map(|g| g.0.clone()) else {
        return;
    };
    manager.update(cx, |this, cx| {
        if this.sources.contains_key(&owner.entity_id()) {
            return;
        }
        let subscription = cx.subscribe(owner, |this, owner, event, cx| match event {
            ConversationEvent::Attention(notice) => this.deliver(&owner, notice, cx),
            ConversationEvent::Changed(changes)
                if changes.selection
                    || changes.catalog
                    || changes.sessions.values().any(|navigation| *navigation) =>
            {
                this.refresh(cx)
            }
            ConversationEvent::Notify { message, error } => {
                // Preserve existing operation feedback on its window when the app
                // is backgrounded. No source session is inferred for these events.
                let window = foreground_window(cx).or_else(|| {
                    cx.windows().into_iter().find(|w| {
                        this.presentations
                            .get(&w.window_id())
                            .is_some_and(|p| p.state.entity_id() == owner.entity_id())
                    })
                });
                this.toast(
                    feedback_text(message, cx),
                    if *error {
                        Severity::Error
                    } else {
                        Severity::Info
                    },
                    None,
                    window,
                    cx,
                );
            }
            _ => {}
        });
        this.sources.insert(
            owner.entity_id(),
            Source {
                state: owner.downgrade(),
                _subscription: subscription,
            },
        );
        this.refresh(cx);
    });
}
/// Called only for visibility/activation changes, not for every token or render.
pub fn present(owner: &Entity<ConversationState>, window: &Window, visible: bool, cx: &mut App) {
    let id = window.window_handle().window_id();
    let owner = owner.downgrade();
    // May be called during a Home/Startup update. Avoid re-entering a window or source.
    cx.defer(move |cx| {
        let Some(manager) = cx.try_global::<Notifications>().map(|g| g.0.clone()) else {
            return;
        };
        manager.update(cx, |this, cx| {
            this.presentations.insert(
                id,
                Presentation {
                    state: owner,
                    visible,
                },
            );
            this.refresh(cx);
        });
    });
}
impl Delivery {
    fn visible(&self, target: &Target, active: Option<AnyWindowHandle>, cx: &App) -> bool {
        active
            .and_then(|w| self.presentations.get(&w.window_id()))
            .is_some_and(|p| {
                p.visible
                    && p.state.entity_id() == target.state.entity_id()
                    && p.state
                        .upgrade()
                        .is_some_and(|s| s.read(cx).selected().as_ref() == Some(&target.key))
            })
    }
    fn toast(
        &self,
        text: String,
        severity: Severity,
        target: Option<(SharedString, Target)>,
        window: Option<AnyWindowHandle>,
        cx: &mut App,
    ) {
        let Some(window) = window else {
            return;
        };
        cx.defer(move |cx| {
            let _ = window.update(cx, |_, window, cx| {
                let notice = match severity {
                    Severity::Info => Notification::info(text),
                    Severity::Warning => Notification::warning(text),
                    Severity::Error => Notification::error(text),
                };
                window.push_notification(
                    if let Some((tag, target)) = target {
                        notice.id1::<Target>(tag).on_click(move |_, _, cx| {
                            let target = target.clone();
                            cx.defer(move |cx| target.open(cx));
                        })
                    } else {
                        notice
                    },
                    cx,
                );
            });
        });
    }
    fn deliver(
        &mut self,
        owner: &Entity<ConversationState>,
        notice: &Notice,
        cx: &mut Context<Self>,
    ) {
        let target = Target {
            state: owner.downgrade(),
            key: notice.key.clone(),
            binding: notice.binding,
            request: match &notice.kind {
                Kind::Waiting(id) => Some(id.clone()),
                _ => None,
            },
        };
        if !target.valid(cx) {
            return;
        }
        let active = foreground_window(cx);
        let visible = self.visible(&target, active, cx);
        let foreground = active.is_some();
        self.serial += 1;
        let identity = match &notice.kind {
            Kind::Waiting(id) => format!("request-{id}"),
            Kind::Completed => "completed".into(),
            Kind::Failed => "failed".into(),
            Kind::Plugin => notice
                .message
                .as_ref()
                .and_then(|m| m.id.as_ref())
                .map(|id| format!("plugin-{id}"))
                .unwrap_or_else(|| format!("plugin-{}", self.serial)),
        };
        let tag: SharedString = format!(
            "gupi-{}-{}-{}-{identity}",
            owner.entity_id(),
            notice.key,
            notice.binding
        )
        .into();
        if self
            .preferences
            .system(&delivery_kind(&notice.kind), foreground, visible)
        {
            self.delivered.insert(
                tag.clone(),
                Delivered {
                    target: target.clone(),
                    window: None,
                },
            );
            cx.show_system_notification(SystemNotification {
                tag,
                title: "Gupi".into(),
                body: t(cx, notice.kind.label()).into(),
                actions: Vec::new(),
            });
        } else if foreground
            && (notice.kind == Kind::Plugin || !visible && notice.kind != Kind::Completed)
        {
            let title = owner
                .read(cx)
                .sessions()
                .get(&notice.key)
                .map(|s| gupi_conversation_ui::home::navigation::display_title(s.info(), cx))
                .unwrap_or_default();
            let text = notice
                .message
                .as_ref()
                .map(|m| m.message.clone())
                .unwrap_or_else(|| t(cx, notice.kind.label()));
            let severity = notice
                .message
                .as_ref()
                .map(|m| m.severity)
                .unwrap_or_else(|| {
                    if notice.kind == Kind::Failed {
                        Severity::Error
                    } else {
                        Severity::Info
                    }
                });
            // A source-visible plugin toast remains long enough to read. Retained
            // source content is still available after the toast disappears.
            if !visible {
                self.delivered.insert(
                    tag.clone(),
                    Delivered {
                        target: target.clone(),
                        window: active,
                    },
                );
            }
            self.toast(
                format!("{title}\n{text}"),
                severity,
                Some((tag, target)),
                active,
                cx,
            );
        }
        if !foreground && self.preferences.attention && matches!(notice.kind, Kind::Waiting(_)) {
            #[cfg(not(test))]
            if let Err(error) = platform_ext::app::request_attention() {
                tracing::warn!(%error, "request application attention failed");
            }
            #[cfg(target_os = "windows")]
            if let Some(window) = cx.windows().first().copied() {
                cx.defer(move |cx| {
                    let _ = window.update(cx, |_, window, _| window.request_attention());
                });
            }
        }
        self.refresh(cx);
    }
    fn refresh(&mut self, cx: &mut Context<Self>) {
        self.sources.retain(|_, s| s.state.upgrade().is_some());
        let windows = cx.windows();
        self.presentations
            .retain(|id, _| windows.iter().any(|w| w.window_id() == *id));
        let active = foreground_window(cx);
        if let Some(p) = active.and_then(|w| self.presentations.get(&w.window_id()))
            && p.visible
            && let Some(owner) = p.state.upgrade()
            && let Some(key) = owner.read(cx).selected().clone()
        {
            owner.update(cx, |s, cx| s.mark_read(&key, cx));
        }
        let stale: Vec<_> = self
            .delivered
            .iter()
            .filter(|(_, delivered)| {
                !delivered.target.valid(cx) || self.visible(&delivered.target, active, cx)
            })
            .map(|(tag, _)| tag.clone())
            .collect();
        for tag in stale {
            if let Some(delivered) = self.delivered.remove(&tag) {
                if let Some(window) = delivered.window {
                    cx.defer(move |cx| {
                        let _ = window.update(cx, |_, window, cx| {
                            window.remove_notification1::<Target>(tag, cx)
                        });
                    });
                } else {
                    cx.dismiss_system_notification(&tag);
                }
            }
        }
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        let mut rows = Vec::new();
        let mut count = 0;
        for source in self.sources.values() {
            if let Some(owner) = source.state.upgrade() {
                let state = owner.read(cx);
                count += state.sessions().values().filter(|s| s.unread()).count();
                #[cfg(any(target_os = "macos", target_os = "windows"))]
                for (key, s) in state.sessions() {
                    if s.unread() || matches!(s.activity(), Activity::Waiting | Activity::Running) {
                        rows.push(TrayRow {
                            owner: owner.entity_id(),
                            key: key.clone(),
                            title: gupi_conversation_ui::home::navigation::display_title(
                                s.info(),
                                cx,
                            ),
                            activity: s.activity(),
                            unread: s.unread(),
                            binding: s.binding(),
                        });
                    }
                }
            }
        }
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        rows.sort_by(|a, b| a.owner.cmp(&b.owner).then(a.key.cmp(&b.key)));
        if count != self.badge {
            tracing::info!(
                previous = self.badge,
                count,
                foreground = active.is_some(),
                "notification unread count changed"
            );
            self.badge = count;
            #[cfg(all(target_os = "macos", not(test)))]
            if count > 0 && self.badge_authorization.is_none() {
                let (tx, rx) = smol::channel::bounded(1);
                platform_ext::app::request_badge_authorization(move |granted| {
                    let _ = tx.try_send(granted);
                });
                self.badge_authorization = Some(cx.spawn(async move |owner, cx| {
                    if let Ok(granted) = rx.recv().await {
                        let _ = owner.update(cx, |this, _| {
                            tracing::info!(granted, "notification badge authorization completed");
                            // Reading the source while the prompt is open may clear
                            // unread. Reapply the current count, never a captured one.
                            if granted
                                && let Err(error) = platform_ext::app::set_badge_count(this.badge)
                            {
                                tracing::warn!(%error, "set application badge failed");
                            }
                        });
                    }
                }));
            }
            #[cfg(not(test))]
            if let Err(error) = platform_ext::app::set_badge_count(count) {
                tracing::warn!(%error, "set application badge failed");
            }
        }
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        if rows != self.tray {
            let entries = rows
                .iter()
                .filter_map(|row| {
                    let source = self.sources.get(&row.owner)?;
                    Some(super::tray::Entry {
                        title: row.title.clone(),
                        activity: row.activity,
                        unread: row.unread,
                        target: Target {
                            state: source.state.clone(),
                            key: row.key.clone(),
                            binding: row.binding,
                            request: None,
                        },
                    })
                })
                .collect();
            self.tray = rows;
            super::tray::update(count, entries, cx);
        }
    }
}

fn delivery_kind(kind: &Kind) -> gupi_settings::notifications::DeliveryKind {
    use gupi_settings::notifications::DeliveryKind;
    match kind {
        Kind::Waiting(_) => DeliveryKind::Waiting,
        Kind::Completed => DeliveryKind::Completed,
        Kind::Failed => DeliveryKind::Failed,
        Kind::Plugin => DeliveryKind::Plugin,
    }
}

fn feedback_text(message: &gupi_conversation::feedback::Feedback, cx: &App) -> String {
    use fluent_bundle::FluentArgs;
    use gupi_conversation::feedback::Feedback;
    let mut args = FluentArgs::new();
    let key = match message {
        Feedback::Message(message) => return message.clone(),
        Feedback::DeleteFailed(error) => {
            args.set("error", error.as_str());
            "conversation-delete-failed"
        }
        Feedback::Exported(path) => {
            args.set("path", path.as_str());
            "conversation-exported"
        }
        Feedback::TemplateUnreadable => "resource-template-unreadable",
        Feedback::TemplateUnusedFiles => "resource-template-unused-files",
        Feedback::TemplateUnclosedQuote => "resource-template-unclosed-quote",
    };
    gupi_settings::i18n::t_with_args(cx, key, &args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::IntoElement;
    use gpui_kit::Render;
    use gpui_kit::SystemNotificationResponse;
    use gpui_kit::TestAppContext;
    use gpui_kit::div;
    use gupi_conversation::conversation::PendingUi;
    use gupi_conversation::conversation::Session;
    use pi_rpc::protocol::ExtensionRequest;
    use pi_rpc::protocol::UiMethod;

    fn setup(cx: &mut TestAppContext) -> Entity<ConversationState> {
        cx.update(|cx| {
            cx.set_app_identity("top.sushao.gupi.test", "Gupi Test");
            gpui_kit::init(cx);
            crate::app::init_capability_hosts(cx);
            app_theme::init(cx);
            gupi_settings::theme::init(cx);
            gupi_settings::i18n::apply(gupi_settings::config::AppLanguage::English, cx);
            gupi_pi_runtime::init(cx);
            init(cx);
        });
        let state = cx.new(|cx| ConversationState::new("unused".into(), cx));
        state.update(cx, |s, _| {
            s.sessions_for_test()
                .insert("a".into(), Session::from_rpc_messages(&[]));
            s.sessions_for_test()
                .insert("b".into(), Session::from_rpc_messages(&[]));
            *s.selected_for_test() = Some("a".into());
        });
        cx.run_until_parked();
        state
    }
    fn emit(state: &Entity<ConversationState>, kind: Kind, cx: &mut TestAppContext) {
        state.update(cx, |_, cx| {
            cx.emit(ConversationEvent::Attention(Notice::new(
                "a".into(),
                0,
                kind,
                None,
            )))
        });
        cx.run_until_parked();
    }
    fn refresh(cx: &mut App) {
        cx.global::<Notifications>()
            .0
            .clone()
            .update(cx, |s, cx| s.refresh(cx));
    }
    fn badge(cx: &TestAppContext) -> usize {
        cx.read(|cx| cx.global::<Notifications>().0.read(cx).badge)
    }
    fn waiting(state: &Entity<ConversationState>, id: &str, cx: &mut TestAppContext) {
        state.update(cx, |s, _| {
            s.sessions_for_test()
                .get_mut("a")
                .unwrap()
                .pending_ui_for_test()
                .push_back(PendingUi::new(
                    ExtensionRequest {
                        id: id.into(),
                        method: UiMethod::Editor {
                            title: "Private question".into(),
                            prefill: None,
                        },
                    },
                    None,
                    String::new(),
                    None,
                ))
        });
        emit(state, Kind::Waiting(id.into()), cx);
    }
    #[gpui_kit::test]
    fn notifications_route_each_request_and_retract_resolved_targets(cx: &mut TestAppContext) {
        let state = setup(cx);
        waiting(&state, "one", cx);
        waiting(&state, "two", cx);
        assert_eq!(cx.shown_system_notifications().len(), 2);
        assert!(
            cx.shown_system_notifications()
                .iter()
                .all(|n| !n.body.contains("Private"))
        );
        assert_eq!(badge(cx), 0);
        let old = cx.shown_system_notifications()[0].tag.clone();
        state.update(cx, |s, _| {
            s.sessions_for_test()
                .get_mut("a")
                .unwrap()
                .pending_ui_for_test()
                .pop_front()
        });
        cx.update(refresh);
        assert_eq!(cx.delivered_system_notifications().len(), 1);
        cx.simulate_system_notification_response(SystemNotificationResponse {
            tag: old,
            action_id: None,
        });
        cx.run_until_parked();
        assert_eq!(
            state.read_with(cx, |s, _| s.selected().clone()),
            Some("a".into())
        );
        state.update(cx, |s, _| {
            *s.sessions_for_test()
                .get_mut("a")
                .unwrap()
                .binding_for_test() += 1
        });
        cx.update(refresh);
        assert!(cx.delivered_system_notifications().is_empty());
    }
    #[gpui_kit::test]
    fn notifications_count_conversations_and_read_without_clearing_waiting(
        cx: &mut TestAppContext,
    ) {
        let state = setup(cx);
        waiting(&state, "one", cx);
        state.update(cx, |s, _| {
            for session in s.sessions_for_test().values_mut() {
                *session.unread_for_test() = true;
            }
            *s.sessions_for_test().get_mut("b").unwrap().error_for_test() =
                Some(gupi_conversation::conversation::SessionError::Response);
        });
        cx.update(refresh);
        assert_eq!(badge(cx), 2);
        state.update(cx, |s, cx| s.mark_read("a", cx));
        cx.update(refresh);
        assert_eq!(badge(cx), 1);
        state.read_with(cx, |s, _| {
            assert_eq!(s.sessions()["a"].activity(), Activity::Waiting)
        });
        // Disabling delivery doesn't discard unread results.
        cx.update(|cx| {
            configure(
                {
                    let mut record = Preferences::default();
                    record.completion = gupi_settings::notifications::CompletionMode::Off;
                    record
                },
                cx,
            )
        });
        assert_eq!(badge(cx), 1);
    }
    struct Page;
    impl Render for Page {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
        }
    }
    #[gpui_kit::test]
    fn notifications_foreground_source_is_read_but_settings_is_not(cx: &mut TestAppContext) {
        let state = setup(cx);
        state.update(cx, |s, _| {
            *s.sessions_for_test()
                .get_mut("a")
                .unwrap()
                .unread_for_test() = true
        });
        let (_, visual) = cx.add_window_view(|window, cx| {
            present(&state, window, false, cx);
            window.activate_window();
            Page
        });
        visual.run_until_parked();
        assert_eq!(badge(visual), 1);
        visual.update(|window, cx| present(&state, window, true, cx));
        visual.run_until_parked();
        assert_eq!(badge(visual), 0);
        emit(&state, Kind::Completed, visual);
        assert!(visual.shown_system_notifications().is_empty());
        visual.deactivate_window();
        emit(&state, Kind::Completed, visual);
        assert_eq!(visual.shown_system_notifications().len(), 1);
    }
    #[gpui_kit::test]
    fn notifications_foreground_other_source_gets_one_retractable_toast(cx: &mut TestAppContext) {
        let state = setup(cx);
        state.update(cx, |s, _| *s.selected_for_test() = Some("b".into()));
        let (_, visual) = cx.add_window_view(|window, cx| {
            present(&state, window, true, cx);
            window.activate_window();
            let page = cx.new(|_| Page);
            gpui_kit::component::Root::new(page, window, cx)
        });
        visual.run_until_parked();
        // Reattaching another view to the same global owner cannot duplicate delivery.
        visual.update(|_, cx| attach(&state, cx));
        waiting(&state, "one", visual);
        assert!(visual.shown_system_notifications().is_empty());
        assert_eq!(visual.update(|w, cx| w.notifications(cx).len()), 1);
        state.update(visual, |s, _| {
            s.sessions_for_test()
                .get_mut("a")
                .unwrap()
                .pending_ui_for_test()
                .clear()
        });
        visual.update(|_, cx| refresh(cx));
        visual
            .background_executor
            .advance_clock(std::time::Duration::from_secs(1));
        visual.run_until_parked();
        assert_eq!(visual.update(|w, cx| w.notifications(cx).len()), 0);
    }
}
