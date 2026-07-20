mod bar;
mod config;
mod dashboard;
mod hotkey;
mod js_runtime;
mod panel;

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use config::load_config;
use dashboard::Dashboard;

use gpui::*;
use gpui_component::{Root, Theme, ThemeMode};

fn main() {
    let config = load_config();
    let app = gpui_platform::application().with_assets(gpui_component_assets::Assets);
    app.run(move |cx| {
        gpui_component::init(cx);

        if let Some(ref bar_config) = config.bar {
            let always_on_top = config.always_on_top.unwrap_or(true);

            // 按 display_index 选择显示器,默认 0(主显示器)
            let displays = cx.displays();
            let display_index = config.display_index.unwrap_or(0);
            let display = displays
                .get(display_index)
                .cloned()
                .unwrap_or_else(|| cx.primary_display().expect("no display"));
            let screen = display.bounds();

            let bar_w = px(360.);
            // 每个 panel 基础高度 56px;info-line 按行数计算(每行 ~22px + 12px padding)
            let bar_h: Pixels = {
                let mut h = 24f32; // 上下窗口 padding
                let n = bar_config.panels.len() as f32;
                for (i, panel) in bar_config.panels.iter().enumerate() {
                    if i > 0 {
                        h += 8.0; // panel 间距
                    }
                    h += match panel {
                        crate::config::BarPanel::InfoLine { title, items } => {
                            // 行高 22px + py(12)*2 = 24px 呼吸空间
                            // 可选 title 占一行(22px)
                            let title_h = if title.is_some() { 22.0 } else { 0.0 };
                            (items.len().max(1) as f32) * 22.0 + title_h + 24.0
                        }
                        _ => 56.0 + 24.0,
                    };
                }
                let _ = n;
                px(h)
            };
            let margin = px(16.);
            let origin = Point {
                x: screen.origin.x + screen.size.width - bar_w - margin,
                y: screen.origin.y + margin,
            };

            let bar_view = cx.new(|_cx| bar::Bar::new(bar_config.clone()));
            let window = cx
                .open_window(
                    WindowOptions {
                        window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                            origin,
                            Size::new(bar_w, bar_h),
                        ))),
                        titlebar: None,
                        is_resizable: false,
                        is_minimizable: false,
                        is_movable: true,
                        display_id: Some(display.id()),
                        kind: if always_on_top {
                            WindowKind::Floating
                        } else {
                            WindowKind::Normal
                        },
                        ..Default::default()
                    },
                    |window, cx| {
                        Theme::change(ThemeMode::Dark, Some(window), cx);
                        cx.new(|cx| Root::new(bar_view.clone(), window, cx))
                    },
                )
                .expect("Failed to open bar window");

            // 非 .app bundle 从命令行启动时,activation policy 在 didFinishLaunching 刚设置,
            // 需等下一个 runloop 让其生效后再激活应用并前置窗口。
            cx.defer({
                let window = window.clone();
                move |cx| {
                    cx.activate(true);
                    window
                        .update(cx, |_, window, _| window.activate_window())
                        .ok();
                }
            });

            // 注册全局热键,唤起/隐藏 bar
            let hotkey_str = config
                .hotkey
                .as_deref()
                .unwrap_or("cmd+shift+b");
            match hotkey::register(hotkey_str) {
                Ok((hotkey_id, manager)) => {
                    // manager 必须保持存活,放到全局状态
                    cx.set_global(HotkeyManagerState(manager));

                    let receiver = global_hotkey::GlobalHotKeyEvent::receiver();
                    let visible = std::sync::Arc::new(AtomicBool::new(true));

                    cx.spawn(async move |cx| {
                        loop {
                            cx.background_executor()
                                .timer(Duration::from_millis(50))
                                .await;

                            while let Ok(event) = receiver.try_recv() {
                                if event.id == hotkey_id
                                    && event.state == global_hotkey::HotKeyState::Pressed
                                {
                                    let visible = visible.clone();
                                    let _ = cx.update(|cx| {
                                        let is_visible = visible.load(Ordering::Relaxed);
                                        if is_visible {
                                            cx.hide();
                                            visible.store(false, Ordering::Relaxed);
                                        } else {
                                            cx.activate(true);
                                            let _ = window.update(cx, |_, window, _| {
                                                window.activate_window();
                                            });
                                            visible.store(true, Ordering::Relaxed);
                                        }
                                    });
                                }
                            }
                        }
                    })
                    .detach();
                }
                Err(e) => {
                    eprintln!("[hotkey] 注册热键 `{hotkey_str}` 失败: {e}");
                }
            }

            // 注册窗口级刷新热键:仅 bar 窗口聚焦时派发 RefreshConfig 动作。
            // keybinding 在 keymap 中全局注册,但动作派发是 focus-based,
            // 故仅在 bar 窗口为 key window 时触发(窗口级别)。
            let refresh_str = config.refresh_hotkey.as_deref().unwrap_or("cmd+r");
            match hotkey::to_gpui_keystroke(refresh_str) {
                Ok(ks) => match gpui::Keystroke::parse(&ks) {
                    Ok(_) => {
                        cx.bind_keys([KeyBinding::new(&ks, bar::RefreshConfig, None)]);
                        let bar_view = bar_view.clone();
                        cx.on_action(move |_: &bar::RefreshConfig, cx: &mut App| {
                            let _ = bar_view.update(cx, |bar, cx| bar.reload_config(cx));
                        });
                    }
                    Err(e) => eprintln!(
                        "[hotkey] refresh hotkey `{refresh_str}` (`{ks}`) invalid: {e:?}"
                    ),
                },
                Err(e) => eprintln!("[hotkey] refresh hotkey `{refresh_str}` invalid: {e}"),
            }
        } else {
            let window = cx
                .open_window(WindowOptions::default(), |window, cx| {
                    Theme::change(ThemeMode::Dark, Some(window), cx);
                    let view = cx.new(|cx| Dashboard::new(config.clone(), cx));
                    cx.new(|cx| Root::new(view, window, cx))
                })
                .expect("Failed to open window");

            cx.defer({
                let window = window.clone();
                move |cx| {
                    cx.activate(true);
                    window
                        .update(cx, |_, window, _| window.activate_window())
                        .ok();
                }
            });
        }
    });
}

/// 持有 GlobalHotKeyManager,保持全局热键注册存活。
struct HotkeyManagerState(#[allow(dead_code)] global_hotkey::GlobalHotKeyManager);

impl gpui::Global for HotkeyManagerState {}
