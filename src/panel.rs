use gpui::*;
use gpui_component::{
    chart::{AreaChart, BarChart, LineChart, PieChart},
    h_flex, v_flex,
    progress::{Progress, ProgressCircle},
    Icon, IconName, StyledExt as _,
};

use crate::config::{DataPoint, PanelConfig, PanelKind};

impl PanelKind {
    pub fn render(
        &self,
        cfg: &PanelConfig,
        theme: &gpui_component::Theme,
    ) -> impl IntoElement {
        let card = v_flex()
            .min_w(px(300.))
            .p_4()
            .border_1()
            .border_color(theme.border)
            .rounded(theme.radius_lg)
            .bg(theme.background)
            .gap_2();

        match self {
            PanelKind::Stat { value, unit, percent } => {
                let dynamic_value = value.unwrap_or(0.0);
                let color = theme.chart_1;
                let pct = percent.unwrap_or(0.0);
                card.child(
                    h_flex()
                        .justify_between()
                        .items_center()
                        .child(
                            h_flex()
                                .gap_2()
                                .items_center()
                                .child(Icon::new(IconName::Cpu).text_color(color))
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(theme.muted_foreground)
                                        .child(cfg.title.clone()),
                                ),
                        )
                        .child(
                            div()
                                .text_2xl()
                                .font_semibold()
                                .child(format!(
                                    "{:.1}{}",
                                    dynamic_value,
                                    unit.as_deref().unwrap_or("")
                                )),
                        ),
                )
                .child(
                    h_flex()
                        .items_center()
                        .gap_2()
                        .child(
                            ProgressCircle::new(format!("pc-{}", cfg.id))
                                .value(pct)
                                .color(color)
                                .size_4(),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child(format!("{:.0}%", pct)),
                        ),
                )
            }
            PanelKind::Progress { value, max } => {
                let dynamic_value = value.unwrap_or(0.0);
                let max = max.unwrap_or(100.0);
                let pct = if max > 0.0 {
                    (dynamic_value / max * 100.0) as f32
                } else {
                    0.0
                };
                card.child(
                    div()
                        .text_sm()
                        .font_semibold()
                        .text_color(theme.foreground)
                        .child(cfg.title.clone()),
                )
                .child(
                    h_flex()
                        .items_center()
                        .gap_2()
                        .child(
                            Progress::new(format!("pg-{}", cfg.id))
                                .value(pct)
                                .color(theme.chart_2),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child(format!("{:.0}%", pct)),
                        ),
                )
            }
            PanelKind::LineChart { data } => {
                card.child(
                    div()
                        .text_sm()
                        .font_semibold()
                        .text_color(theme.foreground)
                        .child(cfg.title.clone()),
                )
                .child(
                    div()
                        .h(px(240.))
                        .child(
                            LineChart::new(data.clone())
                                .x(|d: &DataPoint| d.label.clone())
                                .y(|d: &DataPoint| d.value)
                                .stroke(theme.chart_1)
                                .tick_margin(3)
                                .id(format!("line-{}", cfg.id)),
                        ),
                )
            }
            PanelKind::AreaChart { data } => {
                card.child(
                    div()
                        .text_sm()
                        .font_semibold()
                        .text_color(theme.foreground)
                        .child(cfg.title.clone()),
                )
                .child(
                    div()
                        .h(px(240.))
                        .child(
                            AreaChart::new(data.clone())
                                .x(|d: &DataPoint| d.label.clone())
                                .y(|d: &DataPoint| d.value)
                                .stroke(theme.chart_1)
                                .fill(linear_gradient(
                                    0.,
                                    linear_color_stop(theme.chart_1.opacity(0.4), 1.),
                                    linear_color_stop(theme.background.opacity(0.1), 0.),
                                ))
                                .tick_margin(3)
                                .id(format!("area-{}", cfg.id)),
                        ),
                )
            }
            PanelKind::BarChart { data } => {
                card.child(
                    div()
                        .text_sm()
                        .font_semibold()
                        .text_color(theme.foreground)
                        .child(cfg.title.clone()),
                )
                .child(
                    div()
                        .h(px(240.))
                        .child(
                            BarChart::new(data.clone())
                                .band(|d: &DataPoint| d.label.clone())
                                .value(|d: &DataPoint| d.value)
                                .tick_margin(3)
                                .id(format!("bar-{}", cfg.id)),
                        ),
                )
            }
            PanelKind::PieChart { data } => {
                let colors = [
                    theme.chart_1,
                    theme.chart_2,
                    theme.chart_3,
                    theme.chart_4,
                    theme.chart_5,
                ];
                let color_map: std::collections::HashMap<String, Hsla> = data
                    .iter()
                    .enumerate()
                    .map(|(i, d)| (d.label.clone(), colors[i % colors.len()]))
                    .collect();
                card.child(
                    div()
                        .text_sm()
                        .font_semibold()
                        .text_color(theme.foreground)
                        .child(cfg.title.clone()),
                )
                .child(
                    div()
                        .h(px(240.))
                        .child(
                            PieChart::new(data.clone())
                                .value(|d: &DataPoint| d.value as f32)
                                .pad_angle(0.02)
                                .color(move |d: &DataPoint| {
                                    let fallback = colors[0];
                                    *color_map.get(&d.label).unwrap_or(&fallback)
                                })
                                .label(|d: &DataPoint| d.label.clone().into()),
                        ),
                )
            }
        }
    }
}
