use std::convert::TryFrom;

use gpui::*;
use gpui_component::{h_flex, v_flex, progress::Progress, ActiveTheme, Sizable, StyledExt as _};

use crate::config::{BarAction, BarConfig, BarInfoLineItem, BarPanel, BarStatItem};

pub struct Bar {
    config: BarConfig,
}

impl Bar {
    pub fn new(config: BarConfig) -> Self {
        Self { config }
    }

    fn format_value(value: f64, unit: &Option<String>) -> String {
        let unit_str = unit.as_deref().unwrap_or("");
        if value.fract() == 0.0 {
            format!("{:.0}{}", value, unit_str)
        } else {
            format!("{:.1}{}", value, unit_str)
        }
    }

    fn parse_color(hex: &str) -> Option<Hsla> {
        Rgba::try_from(hex).ok().map(Hsla::from)
    }

    /// 执行点击动作。阻塞型操作(url/function)放到后台线程，避免卡 UI。
    fn execute_action(action: &BarAction, cx: &mut App) {
        match action {
            BarAction::Url { url } => {
                let url = url.clone();
                cx.background_executor()
                    .spawn(async move {
                        if let Err(e) = open::that(&url) {
                            eprintln!("[action:url] open {url} failed: {e}");
                        }
                    })
                    .detach();
            }
            BarAction::Command { command } => {
                match std::process::Command::new("sh").arg("-c").arg(command).spawn() {
                    Ok(_) => {}
                    Err(e) => eprintln!("[action:command] `{command}` spawn failed: {e}"),
                }
            }
            BarAction::Function { name } => {
                let path = crate::config::config_path();
                let name = name.clone();
                cx.background_executor()
                    .spawn(async move {
                        if let Err(e) = crate::js_runtime::call_config_function(&path, &name) {
                            eprintln!("[action:function] call {name} failed: {e}");
                        }
                    })
                    .detach();
            }
        }
    }

    /// 给元素附加 hover 高亮 + 点击动作。返回带交互的元素。
    /// `flex_1`: wrapper 是否撑满父容器(stat-row 的每个 item 需要,progress-bar 整体已 flex_1)。
    fn make_clickable(
        el: impl IntoElement,
        id: ElementId,
        action: Option<&BarAction>,
        theme: &gpui_component::Theme,
        flex_1: bool,
    ) -> Stateful<Div> {
        let mut wrapper = div().id(id).child(el);
        if flex_1 {
            wrapper = wrapper.flex_1();
        }

        if let Some(action) = action {
            let action = action.clone();
            let hover_bg = theme.accent.opacity(0.25);
            let radius = theme.radius;
            wrapper = wrapper
                .cursor_pointer()
                .rounded(radius)
                .hover(move |s| s.bg(hover_bg))
                .on_click(move |_, _, cx| {
                    Self::execute_action(&action, cx);
                });
        }

        wrapper
    }

    fn render_stat_row(
        &self,
        items: &Vec<BarStatItem>,
        panel_idx: usize,
        theme: &gpui_component::Theme,
    ) -> impl IntoElement {
        let item_count = items.len().max(1);
        let mut children: Vec<AnyElement> = Vec::new();

        for (i, item) in items.iter().enumerate() {
            let value_text = Self::format_value(item.value, &item.unit);

            let value_color = item.color.as_deref()
                .and_then(Self::parse_color)
                .unwrap_or(theme.foreground);

            let mut value_div = div()
                .text_lg()
                .font_semibold()
                .text_color(value_color)
                .child(value_text);

            if let Some(font) = &item.font {
                value_div = value_div.font_family(font.clone());
            }

            let item_el = h_flex()
                .h_full()
                .flex_col()
                .justify_center()
                .items_center()
                .gap_0()
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(item.label.clone()),
                )
                .child(value_div);

            let clickable = Self::make_clickable(
                item_el,
                ElementId::named_usize(format!("stat-{panel_idx}"), i),
                item.action.as_ref(),
                theme,
                true,
            );

            children.push(clickable.into_any_element());

            if i < item_count - 1 {
                children.push(
                    div()
                        .w(px(1.))
                        .h(px(28.))
                        .bg(theme.border.opacity(0.4))
                        .into_any_element(),
                );
            }
        }

        h_flex()
            .flex_1()
            .h_full()
            .items_center()
            .gap(px(8.))
            .children(children)
    }

    fn render_progress_bar(
        &self,
        label: &str,
        value: f64,
        max: f64,
        unit: &Option<String>,
        color: &Option<String>,
        font: &Option<String>,
        action: &Option<BarAction>,
        panel_idx: usize,
        theme: &gpui_component::Theme,
    ) -> impl IntoElement {
        let pct = if max > 0.0 {
            ((value / max * 100.0) as f32).clamp(0.0, 100.0)
        } else {
            0.0
        };
        let bar_color = color.as_deref()
            .and_then(Self::parse_color)
            .unwrap_or(theme.chart_1);
        let value_color = color.as_deref()
            .and_then(Self::parse_color)
            .unwrap_or(theme.foreground);

        let display_value = if unit.is_some() {
            format!("{:.1} / {:.1} {}", value, max, unit.as_deref().unwrap())
        } else {
            format!("{:.1}%", pct)
        };

        let mut label_div = div()
            .text_sm()
            .text_color(theme.muted_foreground)
            .child(label.to_string());

        if let Some(f) = font {
            label_div = label_div.font_family(f.clone());
        }

        let mut value_div = div()
            .text_sm()
            .font_semibold()
            .text_color(value_color)
            .child(display_value);

        if let Some(f) = font {
            value_div = value_div.font_family(f.clone());
        }

        let card_el = v_flex()
            .h_full()
            .justify_center()
            .gap(px(6.))
            .px(px(8.))
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .child(label_div)
                    .child(value_div),
            )
            .child(
                div()
                    .w_full()
                    .child(
                        Progress::new("bar-progress")
                            .value(pct)
                            .color(bar_color)
                            .small(),
                    ),
            );

        Self::make_clickable(card_el, ElementId::Name(format!("prog-{panel_idx}").into()), action.as_ref(), theme, true)
    }

    /// 通用:构造带可选字体/颜色的文字 div
    fn text_div(
        content: &str,
        text_size: impl Fn(Div) -> Div,
        color: Hsla,
        font: &Option<String>,
    ) -> Div {
        let mut d = text_size(div()).text_color(color).child(content.to_string());
        if let Some(f) = font {
            d = d.font_family(f.clone());
        }
        d
    }

    /// info-line 列表:每行 title 左 / desc 右,紧凑单行,溢出省略号
    fn render_info_line(
        &self,
        panel_title: &Option<String>,
        items: &[BarInfoLineItem],
        panel_idx: usize,
        theme: &gpui_component::Theme,
    ) -> impl IntoElement {
        let mut rows: Vec<AnyElement> = Vec::new();

        // 可选标题:普通字号(text_xs),非醒目,仅作分组说明。
        // title 行加 py(2) 与 item 行对齐,使列表上下边距对称:
        // 否则首行 title 无 py,顶部内容到分割线 = wrapper py(12) = 12px;
        // 末行 item 有 py(2),底部内容到分割线 = wrapper py(12) + item py(2) = 14px,
        // 视觉上上小下大。加 py(2) 后上下均为 14px。
        if let Some(title) = panel_title {
            rows.push(
                Self::text_div(
                    title,
                    |d| d.text_xs(),
                    theme.muted_foreground,
                    &None,
                )
                .py(px(2.))
                .into_any_element(),
            );
        }

        for (i, item) in items.iter().enumerate() {
            let title_color = item.color.as_deref()
                .and_then(Self::parse_color)
                .unwrap_or(theme.foreground);
            let desc_color = item.desc_color.as_deref()
                .and_then(Self::parse_color)
                .unwrap_or(theme.muted_foreground);

            let mut row = h_flex()
                .w_full()
                .items_center()
                .gap(px(6.))
                // title 固定不收缩、不截断(如 #13025 短编号应完整显示)
                .child(
                    Self::text_div(&item.title, |d| d.text_xs().font_semibold(), title_color, &item.font)
                        .flex_shrink(0.0),
                );

            if let Some(desc_text) = &item.desc {
                row = row.child(
                    Self::text_div(desc_text, |d| d.text_xs(), desc_color, &item.font)
                        .truncate()
                        .flex_shrink(1.0)
                        .min_w(px(0.))
                        .ml_auto(),
                );
            }

            let clickable = Self::make_clickable(
                row.py(px(2.)),
                ElementId::named_usize(format!("info-line-{panel_idx}"), i),
                item.action.as_ref(),
                theme,
                false,
            );
            rows.push(clickable.into_any_element());
        }

        // px(8) 与 progress-bar 的 card_el.px(8) 对齐,
        // 使两个 panel 相对窗口的水平 padding 一致(以 progress-bar 为准)
        v_flex().w_full().px(px(8.)).children(rows)
    }

    fn render_info_block(
        &self,
        title: &str,
        desc: &Option<String>,
        color: &Option<String>,
        desc_color: &Option<String>,
        font: &Option<String>,
        action: &Option<BarAction>,
        panel_idx: usize,
        theme: &gpui_component::Theme,
    ) -> impl IntoElement {
        let title_color = color.as_deref()
            .and_then(Self::parse_color)
            .unwrap_or(theme.foreground);
        let desc_color_resolved = desc_color.as_deref()
            .and_then(Self::parse_color)
            .unwrap_or(theme.muted_foreground);

        let mut col = v_flex()
            .h_full()
            .justify_center()
            .gap(px(2.))
            .px(px(8.))
            // title 单行省略号
            .child(
                Self::text_div(title, |d| d.text_sm().font_semibold(), title_color, font)
                    .truncate(),
            );

        if let Some(desc_text) = desc {
            col = col.child(
                Self::text_div(desc_text, |d| d.text_xs(), desc_color_resolved, font)
                    // desc 允许换行,最多 2 行避免撑太高
                    .line_clamp(2),
            );
        }

        Self::make_clickable(
            col,
            ElementId::Name(format!("info-block-{panel_idx}").into()),
            action.as_ref(),
            theme,
            true,
        )
    }

    fn render_panel(
        &self,
        panel: &BarPanel,
        panel_idx: usize,
        theme: &gpui_component::Theme,
    ) -> AnyElement {
        match panel {
            BarPanel::StatRow { items } => {
                self.render_stat_row(items, panel_idx, theme).into_any_element()
            }
            BarPanel::ProgressBar { label, value, max, unit, color, font, action } => {
                self.render_progress_bar(
                    label, *value, *max, unit, color, font, action, panel_idx, theme,
                )
                .into_any_element()
            }
            BarPanel::InfoLine { title, items } => {
                self.render_info_line(title, items, panel_idx, theme)
                    .into_any_element()
            }
            BarPanel::InfoBlock { title, desc, color, desc_color, font, action } => {
                self.render_info_block(
                    title, desc, color, desc_color, font, action, panel_idx, theme,
                )
                .into_any_element()
            }
        }
    }
}

impl Render for Bar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        // 半透明深色背景，模拟 macOS 菜单栏卡片质感
        let bg = theme.popover.opacity(0.92);
        let border = theme.border.opacity(0.25);

        let mut panels: Vec<AnyElement> = Vec::new();
        let panel_count = self.config.panels.len();

        for (i, panel) in self.config.panels.iter().enumerate() {
            // 每个 panel 上下留 12px 呼吸空间,避免内容紧贴分割线。
            // 12px 取自用户反馈:下边距当前足够,以此为准,上下统一。
            // info-line 列表内部行间距不受影响(py 作用在列表整体外层)。
            panels.push(
                div()
                    .py(px(12.))
                    .child(self.render_panel(panel, i, theme))
                    .into_any_element(),
            );

            if i < panel_count.saturating_sub(1) {
                panels.push(
                    div()
                        .w_full()
                        .h(px(1.))
                        .bg(theme.border.opacity(0.3))
                        .mx(px(16.))
                        .into_any_element(),
                );
            }
        }

        div()
            .size_full()
            .flex()
            .flex_col()
            .px(px(16.))
            .py(px(12.))
            .rounded(theme.radius_lg)
            .bg(bg)
            .border_1()
            .border_color(border)
            .shadow_lg()
            .children(panels)
    }
}
