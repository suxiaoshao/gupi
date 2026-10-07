use gpui_kit::component::ActiveTheme;
use gpui_kit::component::InteractiveElementExt as _;
use gpui_kit::component::Sizable;
use gpui_kit::component::TitleBar;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::*;
use gupi_settings::commands as menus;

pub const HEIGHT: Pixels = px(46.);

pub fn button(id: impl Into<ElementId>) -> Button {
    Button::new(id).ghost().small()
}

/// Stop gestures outside the complete control, so popover triggers still receive
/// their child's events before the titlebar's window-move handlers do.
pub fn control(id: &'static str, child: impl IntoElement) -> AnyElement {
    div()
        .id(id)
        .flex_none()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_double_click(|_, _, cx| cx.stop_propagation())
        .child(child)
        .into_any_element()
}

/// Space for native macOS controls; fullscreen lets AppKit place them itself.
pub fn leading_space(window: &Window) -> Pixels {
    if cfg!(target_os = "macos") && !window.is_fullscreen() {
        px(88.)
    } else if cfg!(target_os = "macos") {
        px(12.)
    } else {
        window.rem_size() * 0.75
    }
}

fn title_bar(cx: &App) -> TitleBar {
    TitleBar::new()
        .pl_0()
        .border_b_0()
        .bg(cx.theme().background)
        .on_close_window(|_, window, cx| {
            window.dispatch_action(Box::new(menus::Quit), cx);
        })
}

/// Page controls share the native title bar only on macOS.
pub fn page_bar(cx: &App) -> impl IntoElement + ParentElement + use<> {
    #[cfg(target_os = "macos")]
    {
        title_bar(cx).h(HEIGHT)
    }
    #[cfg(not(target_os = "macos"))]
    {
        div()
            .w_full()
            .h(HEIGHT)
            .min_h_10()
            .flex_none()
            .bg(cx.theme().background)
    }
}

/// The non-macOS window frame owns the menus, drag space and platform controls.
pub fn window_title_bar(window: &mut Window, cx: &mut App) -> Option<TitleBar> {
    #[cfg(not(target_os = "macos"))]
    {
        let menu = app_menu_bar(window, cx);
        let controls = window.window_controls();
        let controls_width = if cfg!(target_os = "linux")
            && !matches!(window.window_decorations(), Decorations::Client { .. })
        {
            px(0.)
        } else {
            gpui_kit::component::TITLE_BAR_HEIGHT
                * (1 + u32::from(controls.minimize) + u32::from(controls.maximize)) as f32
        };
        // TitleBar's private content row keeps its intrinsic minimum width.
        // Bound only our menu slot using the fixed Kit control width and the
        // platform's supported controls, so its own horizontal scrolling can run.
        let menu_width = (window.viewport_size().width
            - leading_space(window)
            - controls_width
            - window.rem_size() * 4.)
            .max(px(0.));
        Some(
            title_bar(cx)
                // Keep the platform's default height, growing with menu controls at larger rem.
                .min_h_8()
                .pl(if window.is_fullscreen() {
                    // Kit adds its own 0.75rem inset in fullscreen.
                    px(0.)
                } else {
                    leading_space(window)
                })
                .child(
                    div()
                        .id("window-menu")
                        .h_full()
                        .min_w_0()
                        .max_w(menu_width)
                        // Block native drag hit testing as well as title-bar gestures.
                        .occlude()
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .on_double_click(|_, _, cx| cx.stop_propagation())
                        .child(menu),
                )
                .child(div().h_full().flex_1().min_w_16()),
        )
    }
    #[cfg(target_os = "macos")]
    {
        let _ = (window, cx);
        None
    }
}

/// A window retains its own menu interaction/focus state; menu definitions come
/// from the same source as the native macOS menus.
#[cfg(not(target_os = "macos"))]
fn app_menu_bar(
    window: &mut Window,
    cx: &mut App,
) -> Entity<gpui_kit::component::menu::AppMenuBar> {
    use gpui_kit::component::menu::AppMenuBar;
    let state = window.use_keyed_state("gupi-app-menu", cx, |_, cx| {
        let menu = AppMenuBar::new(cx);
        let weak = menu.downgrade();
        let subscription =
            cx.observe_global::<gupi_settings::commands::MenusChanged>(move |_, cx| {
                let _ = weak.update(cx, |menu, cx| menu.reload(cx));
            });
        (menu, subscription)
    });
    state.read(cx).0.clone()
}
