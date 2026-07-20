use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize, Clone, Debug, Default)]
pub struct Config {
    #[serde(default)]
    pub bar: BarConfig,
    /// 启动时是否置顶(浮在其它应用窗口之上),默认 true
    #[serde(default, alias = "alwaysOnTop")]
    pub always_on_top: Option<bool>,
    /// 窗口出现在哪个显示器(0=主显示器,1=第二个...),默认 0
    #[serde(default, alias = "displayIndex")]
    pub display_index: Option<usize>,
    /// 唤起/隐藏 bar 的全局热键,字符串格式如 "cmd+shift+b",默认 "cmd+shift+b"
    #[serde(default, alias = "hotkey")]
    pub hotkey: Option<String>,
    /// 刷新配置的窗口级热键(仅 bar 窗口聚焦时生效),格式如 "cmd+r",默认 "cmd+r"
    #[serde(default, alias = "refreshHotkey")]
    pub refresh_hotkey: Option<String>,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct BarConfig {
    #[serde(default)]
    pub panels: Vec<BarPanel>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum BarPanel {
    StatRow {
        items: Vec<BarStatItem>,
    },
    ProgressBar {
        label: String,
        value: f64,
        max: f64,
        unit: Option<String>,
        /// 颜色，hex 格式如 "#ff5500"，影响进度条和数值颜色。None 时用主题色。
        #[serde(default)]
        color: Option<String>,
        /// 字体名，如 "Helvetica"。None 时用默认字体。
        #[serde(default)]
        font: Option<String>,
        /// 点击动作(整个进度条卡片可点击)
        #[serde(default)]
        action: Option<BarAction>,
    },
    /// 单行信息列表:每行 title 左 / desc 右,两端对齐,溢出省略号
    InfoLine {
        #[serde(default)]
        title: Option<String>,
        items: Vec<BarInfoLineItem>,
    },
    /// 多行信息:title 一行,desc 可换行
    InfoBlock {
        title: String,
        #[serde(default)]
        desc: Option<String>,
        #[serde(default)]
        color: Option<String>,
        #[serde(default)]
        desc_color: Option<String>,
        #[serde(default)]
        font: Option<String>,
        #[serde(default)]
        action: Option<BarAction>,
    },
}

#[derive(Deserialize, Clone, Debug)]
pub struct BarInfoLineItem {
    pub title: String,
    #[serde(default)]
    pub desc: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub desc_color: Option<String>,
    #[serde(default)]
    pub font: Option<String>,
    #[serde(default)]
    pub action: Option<BarAction>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum BarAction {
    /// 在默认浏览器打开 URL
    Url { url: String },
    /// 执行 shell 命令(通过 sh -c)
    Command { command: String },
    /// 调用配置文件中导出的同名函数
    Function { name: String },
}

#[derive(Deserialize, Clone, Debug)]
pub struct BarStatItem {
    pub label: String,
    pub value: f64,
    pub unit: Option<String>,
    /// 颜色，hex 格式如 "#ff5500"，影响数值颜色。None 时用主题色。
    #[serde(default)]
    pub color: Option<String>,
    /// 字体名，如 "Helvetica"。None 时用默认字体。
    #[serde(default)]
    pub font: Option<String>,
    /// 点击动作
    #[serde(default)]
    pub action: Option<BarAction>,
}

pub fn config_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(home).join(".config/gpui-bar/bar.config.ts")
}

pub fn load_config() -> Config {
    let path = config_path();
    match crate::js_runtime::run_config(&path) {
        Ok(value) => serde_json::from_value(value).unwrap_or_else(|e| {
            crate::js_runtime::write_log("[config]", &format!("deserialize failed: {e}"));
            Config::default()
        }),
        Err(e) => {
            crate::js_runtime::write_log("[config]", &format!("run_config failed: {e}"));
            Config::default()
        }
    }
}
