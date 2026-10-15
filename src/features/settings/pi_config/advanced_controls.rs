//! Controls for advanced settings; file and form state stay in PiConfig.
use super::*;
use gpui_kit::component::Sizable as _;
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::{Disableable as _, h_flex};

impl PiConfig {
    pub(super) fn render_timeout(
        &mut self,
        field: Field,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        // Only the edited file's own value; an inherited "disabled" leaves the input editable.
        let disabled = matches!(
            self.snapshot(cx).value(field).first(),
            Some(Slot::Set(value)) if advanced::timeout(value) == Some(0.)
        );
        h_flex()
            .gap_2()
            .items_center()
            .w_full()
            .child(
                gpui_kit::component::input::Input::new(&self.input(field, window, cx))
                    .small()
                    .flex_1()
                    .min_w_0()
                    .aria_label(t(cx, field.label()))
                    .disabled(disabled || self.is_saving()),
            )
            .child(
                Checkbox::new(SharedString::from(format!("pi-disable-{}", field.id())))
                    .label(t(cx, "pi-timeout-disabled"))
                    .checked(disabled)
                    .disabled(self.is_saving())
                    .on_click(cx.listener(move |this, checked: &bool, _, cx| {
                        this.set_values(
                            field,
                            vec![if *checked {
                                Slot::Set(Value::from(0))
                            } else {
                                Slot::Unset
                            }],
                            cx,
                        );
                    })),
            )
            .into_any_element()
    }

    pub(super) fn confirm_always(this: &Entity<Self>, window: &mut Window, cx: &mut App) {
        let this = this.clone();
        window.open_dialog(cx, move |dialog, _, cx| {
            let this = this.clone();
            dialog
                .title(t(cx, "pi-trust-always-title"))
                .child(t(cx, "pi-trust-always-body"))
                .footer(super::super::dialog_buttons(
                    "pi-trust-always-confirm",
                    false,
                    false,
                    cx,
                ))
                .on_ok(move |_, _, cx| {
                    this.update(cx, |this, cx| {
                        this.set_values(
                            Field::ProjectTrust,
                            vec![Slot::Set(Value::from("always"))],
                            cx,
                        )
                    });
                    true
                })
        });
    }
}
