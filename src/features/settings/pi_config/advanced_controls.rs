//! Controls for advanced settings; file and form state stay in PiConfig.
use super::*;
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::{Disableable as _, h_flex};

impl PiConfig {
    pub(super) fn render_advanced(
        &mut self,
        field: Field,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if field == Field::DefaultTools || field == Field::NpmCommand {
            self.render_list(field, window, cx)
        } else {
            self.render_model_map(field, window, cx)
        }
    }
    pub(super) fn render_timeout(
        &mut self,
        field: Field,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let disabled = self.resolved(field, cx).values[0]
            .as_ref()
            .and_then(advanced::timeout)
            == Some(0.);
        h_flex()
            .gap_2()
            .items_center()
            .child(self.render_input(field, window, cx))
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
