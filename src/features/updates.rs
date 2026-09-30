use crate::{
    foundation::{
        i18n::{t, t_with_args},
        releases::Release,
    },
    state::updates::{self, Status, Updates},
};
use fluent_bundle::FluentArgs;
use gpui_kit::component::{
    ActiveTheme, Disableable, WindowExt, button::Button, link::Link, v_flex,
};
use gpui_kit::*;

pub(crate) fn available_text(release: &Release, cx: &App) -> String {
    let mut args = FluentArgs::new();
    args.set("version", release.version.to_string());
    t_with_args(cx, "updates-available", &args)
}

pub(crate) fn open(check: bool, window: &mut Window, cx: &mut App) {
    if window.has_active_dialog(cx) {
        return;
    }
    let owner = updates::get(cx);
    if check {
        owner.update(cx, |owner, cx| owner.check(true, cx));
    }
    let panel = cx.new(UpdatesView::new);
    window.open_dialog(cx, move |dialog, _, cx| {
        dialog.title(t(cx, "updates-title")).child(
            v_flex()
                .gap_3()
                .child(format!("Gupi {}", env!("CARGO_PKG_VERSION")))
                .child(panel.clone()),
        )
    });
}

pub(crate) struct UpdatesView {
    owner: Entity<Updates>,
    _subscription: Subscription,
}

impl UpdatesView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let owner = updates::get(cx);
        let subscription = cx.observe(&owner, |_, _, cx| cx.notify());
        Self {
            owner,
            _subscription: subscription,
        }
    }
}

impl Render for UpdatesView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let owner = self.owner.read(cx);
        let status = match &owner.status {
            Status::Idle => t(cx, "updates-idle"),
            Status::Checking { .. } => t(cx, "updates-checking"),
            Status::Current => t(cx, "updates-current"),
            Status::Unpublished => t(cx, "updates-unpublished"),
            Status::Available(release) => available_text(release, cx),
            #[cfg(any(target_os = "macos", target_os = "windows"))]
            Status::Installing(_) => t(cx, "updates-native-progress"),
            #[cfg(any(target_os = "macos", target_os = "windows"))]
            Status::InstallFailed(_) => t(cx, "updates-install-failed"),
            Status::Failed(problem) => t(cx, problem.key()),
        };
        let mut view = v_flex()
            .gap_2()
            .items_start()
            .child(div().text_sm().child(status));
        if let Some(release) = owner.installable_release() {
            if crate::app::updater::available(cx) {
                view = view.child(
                    Button::new("updates-install")
                        .label(t(cx, "updates-install"))
                        .on_click(|_, _, cx| cx.defer(crate::app::updater::install)),
                );
            }
            view = view
                .child(
                    Link::new("updates-release")
                        .href(release.url.clone())
                        .child(t(cx, "updates-release")),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(t(
                            cx,
                            if crate::app::updater::available(cx) {
                                "updates-restart-help"
                            } else {
                                "updates-install-help"
                            },
                        )),
                );
        }
        let checking = owner.is_checking();
        let installing = owner.is_installing();
        if installing {
            view = view.child(
                Button::new("updates-show-progress")
                    .label(t(cx, "updates-show-progress"))
                    .on_click(|_, _, cx| cx.defer(crate::app::updater::install)),
            );
        }
        view.child(
            Button::new("updates-check")
                .label(t(cx, "updates-check"))
                .loading(checking)
                .disabled(checking || installing)
                .on_click(cx.listener(|this, _, _, cx| {
                    this.owner.update(cx, |owner, cx| owner.check(true, cx));
                })),
        )
    }
}
