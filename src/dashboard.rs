use gpui::*;
use gpui_component::{h_flex, v_flex, ActiveTheme as _, Icon, IconName, StyledExt as _, sidebar::*};
use std::time::Duration;

use crate::config::{DashboardConfig, parse_icon};

pub struct Dashboard {
    config: DashboardConfig,
    current_page: usize,
}

impl Dashboard {
    pub fn new(config: DashboardConfig, cx: &mut Context<Self>) -> Self {
        let mut me = Self { config, current_page: 0 };
        me.start_refresh_loop(cx);
        me
    }

    fn start_refresh_loop(&mut self, cx: &mut Context<Self>) {
        let interval = self.config.refresh_interval.unwrap_or(60);
        let path = crate::config::config_path();
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(interval)).await;
                let path = path.clone();
                let result: anyhow::Result<DashboardConfig> = cx.background_spawn(async move {
                    let value = crate::js_runtime::run_config(&path)?;
                    serde_json::from_value(value).map_err(Into::into)
                }).await;
                match result {
                    Ok(config) => {
                        let _ = this.update(cx, |this, cx| {
                            this.config = config;
                            cx.notify();
                        });
                    }
                    Err(e) => eprintln!("refresh config failed: {e}"),
                }
            }
        }).detach();
    }

    fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let this = cx.entity();

        Sidebar::new("sidebar")
            .w(px(220.))
            .header(
                SidebarHeader::new().child(
                    h_flex()
                        .gap_2()
                        .child(Icon::new(IconName::LayoutDashboard))
                        .child(
                            div()
                                .text_sm()
                                .font_semibold()
                                .child(
                                    self.config
                                        .title
                                        .clone()
                                        .unwrap_or_else(|| "Dashboard".to_string()),
                                ),
                        ),
                ),
            )
            .child(
                SidebarGroup::new("Pages").child(
                    SidebarMenu::new().children(
                        self.config.pages.iter().enumerate().map(|(i, page)| {
                            SidebarMenuItem::new(page.title.clone())
                                .icon(parse_icon(page.icon.as_deref().unwrap_or("layout-dashboard")))
                                .active(i == self.current_page)
                                .on_click({
                                    let this = this.clone();
                                    move |_, _, cx| {
                                        let _ = this.update(cx, |this, cx| {
                                            this.current_page = i;
                                            cx.notify();
                                        });
                                    }
                                })
                        }),
                    ),
                ),
            )
            .footer(
                SidebarFooter::new().child(
                    h_flex()
                        .gap_2()
                        .child(Icon::new(IconName::Settings))
                        .child("Settings"),
                ),
            )
    }

    fn render_page(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let page = self.config.pages.get(self.current_page);

        if let Some(page) = page {
            let mut col: Vec<AnyElement> = Vec::new();
            for panel in &page.panels {
                col.push(panel.kind.render(panel, theme).into_any_element());
            }

            v_flex()
                .flex_1()
                .h_full()
                .min_w_0()
                .p_4()
                .gap_4()
                .child(
                    div()
                        .text_xl()
                        .font_semibold()
                        .text_color(theme.foreground)
                        .child(page.title.clone()),
                )
                .children(col)
        } else {
            v_flex()
                .flex_1()
                .h_full()
                .min_w_0()
                .p_4()
                .child(
                    div()
                        .text_xl()
                        .font_semibold()
                        .text_color(theme.foreground)
                        .child("No pages configured"),
                )
        }
    }
}

impl Render for Dashboard {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        h_flex()
            .size_full()
            .bg(theme.background)
            .child(self.render_sidebar(cx))
            .child(self.render_page(cx))
    }
}
