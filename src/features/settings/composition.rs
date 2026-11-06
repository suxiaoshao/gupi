//! Fixed settings descriptions shared by navigation, filtering and presentation.
use super::*;
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Icon;
use gpui_kit::component::group_box::{GroupBox, GroupBoxVariants as _};
use gpui_kit::component::separator::Separator;
use gpui_kit::component::table::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};
use std::rc::Rc;

#[derive(Clone, Copy)]
pub(super) struct PageLayout {
    pub axis: Axis,
    pub width: Pixels,
    rem: Pixels,
}
impl PageLayout {
    pub fn new(width: Pixels, rem: Pixels) -> Self {
        let axis = if width >= rem * 27.5 {
            Axis::Horizontal
        } else {
            Axis::Vertical
        };
        Self { axis, width, rem }
    }

    pub fn layout(&self) -> Axis {
        self.axis
    }
    pub fn content_width(&self) -> Pixels {
        self.width.min(self.rem * 15.25)
    }
}

/// Product presentation only. Controls and Form retain their existing state.
pub(super) fn settings_row(
    layout: &PageLayout,
    label: impl IntoElement,
    help: Option<SharedString>,
    content: impl IntoElement,
    expanded: bool,
    cx: &App,
) -> AnyElement {
    let stacked = expanded || layout.axis == Axis::Vertical;
    h_flex()
        .w_full()
        .min_w_0()
        .gap_6()
        .py_4()
        .when(stacked, |row| row.flex_col().items_start().gap_3())
        .child(
            v_flex()
                .flex_1()
                .min_w_0()
                .gap_1()
                .when(stacked, |label| label.w_full().flex_none())
                .child(div().text_sm().font_medium().child(label))
                .when_some(help, |label, help| {
                    label.child(
                        div()
                            .text_sm()
                            .font_normal()
                            .whitespace_normal()
                            .text_color(cx.theme().muted_foreground)
                            .child(help),
                    )
                }),
        )
        .child(
            v_flex()
                .min_w_0()
                .flex_shrink_0()
                .items_end()
                .when(!stacked, |control| control.max_w(relative(0.5)))
                .when(stacked, |control| control.w_full().items_start())
                .child(content),
        )
        .into_any_element()
}

type Content = Rc<dyn Fn(&PageLayout, &mut Window, &mut App) -> AnyElement>;
pub(super) fn control<E: IntoElement>(
    render: impl Fn(&PageLayout, &mut Window, &mut App) -> E + 'static,
) -> Content {
    Rc::new(move |layout, window, cx| render(layout, window, cx).into_any_element())
}

pub(super) struct SettingsRow {
    label: Option<(SharedString, SharedString)>,
    description: Option<SharedString>,
    keywords: Vec<SharedString>,
    content: Content,
    framed: bool,
    leading: Option<Content>,
}
impl SettingsRow {
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        content: Content,
    ) -> Self {
        Self {
            label: Some((id.into(), label.into())),
            description: None,
            keywords: vec![],
            content,
            framed: true,
            leading: None,
        }
    }
    pub fn content<E: IntoElement>(
        render: impl Fn(&PageLayout, &mut Window, &mut App) -> E + 'static,
    ) -> Self {
        Self {
            label: None,
            description: None,
            keywords: vec![],
            content: control(render),
            framed: false,
            leading: None,
        }
    }
    pub fn framed(mut self, framed: bool) -> Self {
        self.framed = framed;
        self
    }
    pub fn leading(mut self, content: Content) -> Self {
        self.leading = Some(content);
        self
    }
    pub fn description(mut self, value: impl Into<SharedString>) -> Self {
        self.description = Some(value.into());
        self
    }
    pub fn keywords(mut self, values: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.keywords = values.into_iter().map(Into::into).collect();
        self
    }
    fn matches(&self, query: &str) -> bool {
        query.is_empty()
            || self
                .label
                .as_ref()
                .is_some_and(|(_, title)| contains(title, query))
            || self
                .description
                .as_ref()
                .is_some_and(|text| contains(text, query))
            || self.keywords.iter().any(|text| contains(text, query))
    }
    fn render(self, layout: &PageLayout, window: &mut Window, cx: &mut App) -> AnyElement {
        let content = (self.content)(layout, window, cx);
        if let Some((id, label)) = self.label {
            let row = settings_row(layout, label, self.description, content, false, cx);
            let row = if let Some(leading) = self.leading {
                h_flex()
                    .w_full()
                    .min_w_0()
                    .gap_2()
                    .child(leading(layout, window, cx))
                    .child(div().flex_1().min_w_0().child(row))
                    .into_any_element()
            } else {
                row
            };
            div()
                .id(SharedString::from(format!("settings-row-{id}")))
                .w_full()
                .min_w_0()
                .child(row)
                .into_any_element()
        } else {
            content
        }
    }
    fn render_binding_row(
        self,
        layout: &PageLayout,
        window: &mut Window,
        cx: &mut App,
    ) -> TableRow {
        let title = self.label.map(|(_, label)| label).unwrap_or_default();
        let content = (self.content)(layout, window, cx);
        let stacked = layout.width < layout.rem * 28.75;
        TableRow::new()
            .min_h(rems(4.625))
            .gap_5()
            .py_3()
            .when(stacked, |row| row.flex_col().items_start().gap_2())
            .child(
                TableCell::new()
                    .min_w_0()
                    .flex_1()
                    .px_0()
                    .py_0()
                    .whitespace_normal()
                    .when(stacked, |cell| cell.w_full().flex_none())
                    .child(
                        h_flex()
                            .min_w_0()
                            .gap_2()
                            .items_start()
                            .children(self.leading.map(|leading| leading(layout, window, cx)))
                            .child(
                                v_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .gap_1()
                                    .child(div().text_sm().font_medium().child(title))
                                    .when_some(self.description, |label, help| {
                                        label.child(
                                            div()
                                                .text_xs()
                                                .font_normal()
                                                .whitespace_normal()
                                                .text_color(cx.theme().muted_foreground)
                                                .child(help),
                                        )
                                    }),
                            ),
                    ),
            )
            .child(
                TableCell::new()
                    .min_w_0()
                    .w(relative(0.48))
                    .flex_none()
                    .px_0()
                    .py_0()
                    .whitespace_normal()
                    .when(stacked, |cell| cell.w_full())
                    .child(content),
            )
    }
}

#[derive(Default)]
pub(super) struct SettingsSection {
    title: Option<SharedString>,
    description: Option<SharedString>,
    rows: Vec<SettingsRow>,
    bindings: bool,
    tools: Option<Content>,
}
impl SettingsSection {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn title(mut self, value: impl Into<SharedString>) -> Self {
        self.title = Some(value.into());
        self
    }
    pub fn description(mut self, value: impl Into<SharedString>) -> Self {
        self.description = Some(value.into());
        self
    }
    pub fn item(mut self, row: SettingsRow) -> Self {
        self.rows.push(row);
        self
    }
    pub fn items(mut self, rows: impl IntoIterator<Item = SettingsRow>) -> Self {
        self.rows.extend(rows);
        self
    }
    pub fn tools(
        mut self,
        render: impl Fn(&PageLayout, &mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        self.tools = Some(control(render));
        self
    }
    pub fn bindings(mut self) -> Self {
        self.bindings = true;
        self
    }
    fn heading_matches(&self, query: &str) -> bool {
        self.title
            .as_ref()
            .is_some_and(|text| contains(text, query))
            || self
                .description
                .as_ref()
                .is_some_and(|text| contains(text, query))
    }
}

pub(super) struct SettingsPage {
    pub title: SharedString,
    pub icon: Option<Icon>,
    sections: Vec<SettingsSection>,
    header: Option<Content>,
    scope: Option<Content>,
    actions: Option<Content>,
    description: Option<SharedString>,
    bounded_body: Option<Content>,
}
impl SettingsPage {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            icon: None,
            sections: vec![],
            header: None,
            scope: None,
            actions: None,
            description: None,
            bounded_body: None,
        }
    }
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = title.into();
        self
    }
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }
    pub fn group(mut self, section: SettingsSection) -> Self {
        self.sections.push(section);
        self
    }
    pub fn groups(mut self, sections: impl IntoIterator<Item = SettingsSection>) -> Self {
        self.sections.extend(sections);
        self
    }
    pub fn header<E: IntoElement>(
        mut self,
        render: impl Fn(&PageLayout, &mut Window, &mut App) -> E + 'static,
    ) -> Self {
        self.header = Some(control(render));
        self
    }
    pub fn scope<E: IntoElement>(
        mut self,
        render: impl Fn(&PageLayout, &mut Window, &mut App) -> E + 'static,
    ) -> Self {
        self.scope = Some(control(render));
        self
    }
    pub fn actions<E: IntoElement>(
        mut self,
        render: impl Fn(&PageLayout, &mut Window, &mut App) -> E + 'static,
    ) -> Self {
        self.actions = Some(control(render));
        self
    }
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }
    pub fn bounded_body<E: IntoElement>(
        mut self,
        render: impl Fn(&PageLayout, &mut Window, &mut App) -> E + 'static,
    ) -> Self {
        self.bounded_body = Some(control(render));
        self
    }
    pub fn matches(&self, query: &str) -> bool {
        query.is_empty()
            || contains(&self.title, query)
            || self.sections.iter().any(|section| {
                section.heading_matches(query) || section.rows.iter().any(|row| row.matches(query))
            })
    }
    pub fn content(self, query: String, layout: PageLayout) -> PageContent {
        PageContent {
            page: self,
            query,
            layout,
        }
    }
}

#[derive(IntoElement)]
pub(super) struct PageContent {
    page: SettingsPage,
    query: String,
    layout: PageLayout,
}
impl RenderOnce for PageContent {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let whole_page = self.query.is_empty() || contains(&self.page.title, &self.query);
        let bounded = self.page.bounded_body.is_some();
        let mut body = v_flex()
            .w_full()
            .min_w_0()
            .gap_6()
            .when(bounded, |body| body.flex_1().min_h_0().gap_3());
        let mut heading = h_flex()
            .w_full()
            .min_w_0()
            .flex_wrap()
            .justify_between()
            .gap_3();
        let mut title = h_flex()
            .flex_1()
            .min_w_0()
            .flex_wrap()
            .gap_3()
            .child(div().text_2xl().font_semibold().child(self.page.title));
        if let Some(scope) = self.page.scope {
            title = title.child(div().flex_none().w(rems(9.)).child(scope(
                &self.layout,
                window,
                cx,
            )));
        }
        heading = heading.child(title);
        if let Some(actions) = self.page.actions {
            heading = heading.child(actions(&self.layout, window, cx));
        }
        body = body.child(
            v_flex()
                .when(bounded, |body| body.flex_shrink_0())
                .gap_2()
                .child(heading)
                .when_some(self.page.description, |header, description| {
                    header.child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(description),
                    )
                }),
        );
        if let Some(header) = self.page.header {
            let header = header(&self.layout, window, cx);
            body = if bounded {
                body.child(div().flex_shrink_0().child(header))
            } else {
                body.child(header)
            };
        }
        if let Some(render) = self.page.bounded_body {
            return body.child(div().flex_1().min_h_0().min_w_0().child(render(
                &self.layout,
                window,
                cx,
            )));
        }
        for (index, section) in self.page.sections.into_iter().enumerate() {
            let whole_section = whole_page || section.heading_matches(&self.query);
            let rows = section
                .rows
                .into_iter()
                .filter(|row| whole_section || row.matches(&self.query))
                .collect::<Vec<_>>();
            if rows.is_empty() {
                continue;
            }
            let mut group = v_flex()
                .id(("settings-section", index))
                .w_full()
                .min_w_0()
                .gap_3()
                .when(!section.bindings, |group| {
                    group.when_some(section.title.clone(), |group, title| {
                        group.child(div().font_semibold().child(title))
                    })
                });
            if section.bindings {
                let stacked = self.layout.width < self.layout.rem * 28.75;
                group = group
                    .child(
                        h_flex()
                            .w_full()
                            .min_w_0()
                            .flex_wrap()
                            .justify_between()
                            .gap_3()
                            .child(
                                v_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .gap_1()
                                    .children(
                                        section
                                            .title
                                            .map(|title| div().font_semibold().child(title)),
                                    )
                                    .children(section.description.map(|text| {
                                        div()
                                            .text_sm()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(text)
                                    })),
                            )
                            .children(section.tools.map(|tools| tools(&self.layout, window, cx))),
                    )
                    .child(
                        Table::new()
                            .accessibility_label(crate::features::settings::t(
                                cx,
                                "settings-page-keys",
                            ))
                            .when(!stacked, |table| {
                                table.child(
                                    TableHeader::new().child(
                                        TableRow::new()
                                            .gap_5()
                                            .child(
                                                TableHead::new().flex_1().min_w_0().px_0().child(
                                                    crate::features::settings::t(
                                                        cx,
                                                        "settings-key-table-command",
                                                    ),
                                                ),
                                            )
                                            .child(
                                                TableHead::new()
                                                    .w(relative(0.48))
                                                    .min_w_0()
                                                    .flex_none()
                                                    .px_0()
                                                    .child(crate::features::settings::t(
                                                        cx,
                                                        "settings-key-table-binding",
                                                    )),
                                            ),
                                    ),
                                )
                            })
                            .child(
                                TableBody::new().children(
                                    rows.into_iter().map(|row| {
                                        row.render_binding_row(&self.layout, window, cx)
                                    }),
                                ),
                            ),
                    );
                body = body.child(group);
                continue;
            }
            let mut framed = Vec::new();
            for row in rows {
                if row.framed {
                    if !framed.is_empty() {
                        framed.push(Separator::horizontal().into_any_element());
                    }
                    framed.push(row.render(&self.layout, window, cx));
                } else {
                    if !framed.is_empty() {
                        group = group.child(filled_rows(std::mem::take(&mut framed)));
                    }
                    group = group.child(row.render(&self.layout, window, cx));
                }
            }
            if !framed.is_empty() {
                group = group.child(filled_rows(framed));
            }
            group = group.when_some(section.description, |group, text| {
                group.child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(text),
                )
            });
            body = body.child(group);
        }
        body
    }
}
fn filled_rows(rows: Vec<AnyElement>) -> GroupBox {
    let mut content = v_flex().w_full().min_w_0().px_5().py_0().gap_0();
    GroupBox::new()
        .min_w_0()
        .fill()
        .content_style(content.style().clone())
        .children(rows)
}
fn contains(text: &str, query: &str) -> bool {
    text.to_lowercase().contains(query)
}
