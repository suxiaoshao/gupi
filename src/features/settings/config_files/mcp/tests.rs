use super::{Cancellation, File, FilePages, Kind, Mount, Running, Scope};
use gpui_kit::component::{
    Root, WindowExt as _,
    setting::{SettingGroup, SettingItem, SettingPage, Settings},
};
use gpui_kit::test::TestWindowExt as _;
use gpui_kit::{
    AppContext as _, Context, Entity, IntoElement, Render, TestAppContext, Window, div,
};
use std::{cell::RefCell, rc::Rc};
struct Host {
    mount: Rc<RefCell<Option<Cancellation>>>,
    files: Entity<FilePages>,
    _subscription: gpui_kit::Subscription,
}
impl Render for Host {
    fn render(&mut self, _: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let owner = self.files.clone();
        let mount = self.mount.clone();
        Settings::new("mcp-operation-test")
            .page(
                SettingPage::new("MCP").group(SettingGroup::new().item(SettingItem::render(
                    move |_, window, cx| {
                        let page = owner.update(cx, |this, cx| {
                            this.render_page_with_mcp(
                                Kind::Mcp,
                                (Some("fake-pi".into()), true),
                                window,
                                cx,
                            )
                        });
                        *mount.borrow_mut() = Some(
                            window
                                .use_keyed_state("mcp-cli-page-mount", cx, |_, _| Mount::default())
                                .read(cx)
                                .0
                                .clone(),
                        );
                        page
                    },
                ))),
            )
            .page(
                SettingPage::new("Other")
                    .group(SettingGroup::new().item(SettingItem::render(|_, _, _| div()))),
            )
    }
}
#[gpui_kit::test]
fn mcp_saved_actions_dirty_guard_logout_and_mount_cancellation(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        gupi_settings::i18n::apply(gupi_settings::config::AppLanguage::English, cx);
    });
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mcp.json");
    std::fs::write(&path,r#"{"mcpServers":{"oauth":{"url":"https://fixture.invalid"},"stdio":{"command":"not-executed"},"--help":{"url":"https://fixture.invalid"}}}"#).unwrap();
    let mut owner = None;
    let mounted_token = Rc::new(RefCell::new(None));
    let (_, cx) = cx.add_window_view(|window, cx| {
        let files = cx.new(|cx| {
            let mut files = FilePages::new(cx);
            files.agent = Ok(dir.path().into());
            files.mcp.file = Some(File::load(Kind::Mcp, &path).unwrap());
            files
        });
        owner = Some(files.clone());
        let host = cx.new(|cx| {
            let subscription = cx.observe(&files, |_, _, cx| cx.notify());
            Host {
                files,
                mount: mounted_token.clone(),
                _subscription: subscription,
            }
        });
        Root::new(host, window, cx)
    });
    let owner = owner.unwrap();
    cx.update(|window, cx| {
        window.render_frame(cx);
        assert_eq!(window.find("mcp-check").label(), Some("Check connection"));
        window.click("file-menu-oauth", cx);
    });
    cx.run_until_parked();
    cx.update(|window, cx| {
        window.render_frame(cx);
        assert_eq!(
            window.within("popup-menu").find(0usize).label(),
            Some("Log in")
        );
        assert_eq!(
            window.within("popup-menu").find(1usize).label(),
            Some("Log out")
        );
        window.within("popup-menu").click(1usize, cx);
    });
    cx.run_until_parked();
    cx.update(|window, cx| {
        window.render_frame(cx);
        assert!(window.has_active_dialog(cx));
        assert_eq!(window.find("confirm").label(), Some("Log out"));
        window.close_dialog(cx);
    });
    owner.update(cx, |this, cx| {
        let entry = this.mcp.entry("oauth").unwrap();
        this.mcp.edit("new-draft".into(), Some(entry));
        cx.notify();
    });
    cx.run_until_parked();
    cx.update(|window, cx| {
        window.render_frame(cx);
        assert!(owner.read(cx).dirty());
        window.click("mcp-check", cx);
        assert!(owner.read(cx).mcp_actions.running.is_none());
        window.click("file-menu-oauth", cx);
    });
    cx.run_until_parked();
    cx.update(|window, cx| {
        window.render_frame(cx);
        window.within("popup-menu").click(0usize, cx);
        assert!(!window.has_active_dialog(cx));
        assert!(owner.read(cx).mcp_actions.running.is_none());
        window.press("escape", cx);
    });
    cx.run_until_parked();
    owner.update(cx, |this, cx| {
        this.mcp.edits.clear();
        cx.notify();
    });
    cx.run_until_parked();
    cx.update(|window, cx| window.click("file-menu---help", cx));
    cx.run_until_parked();
    cx.update(|window, cx| {
        window.render_frame(cx);
        assert_eq!(
            window.within("popup-menu").find(0usize).label(),
            Some("Pi cannot log in or out with this name through the CLI")
        );
        window.press("escape", cx);
    });
    cx.run_until_parked();
    // The actual keyed mount must cancel operations added later in the same mount.
    let token = mounted_token.borrow().as_ref().unwrap().child();
    assert!(!token.is_cancelled());
    cx.update(|window, cx| window.within("settings-sidebar").click("0-1", cx));
    cx.run_until_parked();
    cx.update(|window, cx| window.render_frame(cx));
    assert!(token.is_cancelled());
    cx.update(|window, cx| window.within("settings-sidebar").click("0-0", cx));
    cx.run_until_parked();
    cx.update(|window, cx| window.render_frame(cx));
    let reentry = mounted_token.borrow().as_ref().unwrap().child();
    assert!(!reentry.is_cancelled());
    assert!(token.is_cancelled());
    owner.update(cx, |this, cx| {
        this.mcp_actions.message = Some("pi-mcp-completed");
        this.mcp_actions.invalidate();
        assert!(this.mcp_actions.report.is_none() && this.mcp_actions.message.is_none());
        this.mcp_actions.running = Some(Running {
            generation: 99,
            cancellation: reentry.clone(),
        });
        assert!(this.busy());
        this.set_scope(Scope::Project(dir.path().into()), cx);
        assert!(reentry.is_cancelled());
        assert!(
            this.busy(),
            "cleanup must finish before another operation can start"
        );
    });
}

#[cfg(unix)]
#[gpui_kit::test]
async fn mcp_check_projection_invalidation_and_late_scope_completion(cx: &mut TestAppContext) {
    use super::{Action, Outcome, pi_mcp};
    use smol::stream::StreamExt as _;
    use std::os::unix::fs::PermissionsExt as _;
    cx.update(|cx| {
        gpui_kit::init(cx);
        gpui_tokio::init(cx);
        gupi_settings::i18n::apply(gupi_settings::config::AppLanguage::English, cx);
    });
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mcp.json");
    std::fs::write(&path,r#"{"mcpServers":{"oauth":{"url":"https://fixture.invalid"},"missing":{"url":"https://fixture.invalid"}}}"#).unwrap();
    let executable = dir.path().join("fake-pi");
    std::fs::write(&executable,r##"#!/bin/sh
printf '%s' '{"servers":[{"name":"oauth","scope":"global","enabled":true,"state":"connected","tools":["PRIVATE-TOOL"],"transport":"PRIVATE-TRANSPORT"}],"errors":[]}'
"##).unwrap();
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
    let mut owner = None;
    let (_, cx) = cx.add_window_view(|window, cx| {
        let files = cx.new(|cx| {
            let mut files = FilePages::new(cx);
            files.agent = Ok(dir.path().into());
            files.mcp.file = Some(File::load(Kind::Mcp, &path).unwrap());
            files
        });
        owner = Some(files.clone());
        let host = cx.new(|cx| {
            let subscription = cx.observe(&files, |_, _, cx| cx.notify());
            Host {
                files,
                mount: Rc::new(RefCell::new(None)),
                _subscription: subscription,
            }
        });
        Root::new(host, window, cx)
    });
    let owner = owner.unwrap();
    let mount = Cancellation::new();
    let command = executable.to_string_lossy().into_owned();
    cx.update(|window, cx| {
        assert!(
            owner
                .update(cx, |this, cx| this.start_mcp(
                    Action::Check,
                    command.clone(),
                    mount.clone(),
                    window,
                    cx
                ))
                .is_some()
        );
        assert!(
            owner
                .update(cx, |this, cx| this.start_mcp(
                    Action::Check,
                    command.clone(),
                    mount.clone(),
                    window,
                    cx
                ))
                .is_none()
        );
    });
    cx.executor().allow_parking();
    let mut notifications = cx.notifications(&owner);
    while owner.read_with(cx, |this, _| this.mcp_actions.running.is_some()) {
        notifications.next().await.expect("operation owner alive");
    }
    owner.read_with(cx, |this, cx| {
        assert!(this.mcp_actions.report.is_some());
        assert_eq!(
            this.mcp_row_status("oauth", cx),
            "Connected (\u{2068}1\u{2069} tool)"
        );
        assert_eq!(this.mcp_row_status("missing", cx), "Not checked");
    });
    cx.update(|window, cx| {
        window.render_frame(cx);
        assert_eq!(
            window.find("file-row-oauth").label(),
            Some("oauth · Connected (\u{2068}1\u{2069} tool)")
        );
    });
    // A fresh operation invalidates the previous report immediately. Scope change
    // cancels it and its late completion may only release the busy state.
    cx.update(|window, cx| {
        owner.update(cx, |this, cx| {
            this.start_mcp(Action::Check, command.clone(), mount.clone(), window, cx);
            assert!(this.mcp_actions.report.is_none());
            this.set_scope(Scope::Project(dir.path().into()), cx);
        })
    });
    cx.executor().allow_parking();
    let mut notifications = cx.notifications(&owner);
    while owner.read_with(cx, |this, _| this.mcp_actions.running.is_some()) {
        notifications.next().await.expect("operation owner alive");
    }
    owner.read_with(cx, |this, cx| {
        assert!(this.mcp_actions.report.is_none() && this.mcp_actions.message.is_none());
        assert_eq!(this.mcp_row_status("oauth", cx), "Not checked");
    });
    // Reserved help tokens are rejected at the backend boundary, independently
    // of whether a UI menu could accidentally invoke them.
    let worker = cx.update(|_, cx| {
        gpui_tokio::Tokio::spawn(
            cx,
            pi_mcp::run(
                command,
                Action::Login("--help".into()),
                Scope::Global,
                vec![],
                Cancellation::new(),
            ),
        )
    });
    assert_eq!(worker.await.unwrap(), Outcome::Failed);
}
