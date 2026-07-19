use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize, Clone, Debug, Default)]
pub struct DashboardConfig {
    pub title: Option<String>,
    #[serde(default, alias = "refreshInterval")]
    pub refresh_interval: Option<u64>,
    #[serde(default)]
    pub pages: Vec<PageConfig>,
    pub bar: Option<BarConfig>,
    /// 启动时是否置顶(浮在其它应用窗口之上),默认 true
    #[serde(default, alias = "alwaysOnTop")]
    pub always_on_top: Option<bool>,
    /// 窗口出现在哪个显示器(0=主显示器,1=第二个...),默认 0
    #[serde(default, alias = "displayIndex")]
    pub display_index: Option<usize>,
    /// 唤起/隐藏 bar 的全局热键,字符串格式如 "cmd+shift+b",默认 "cmd+shift+b"
    #[serde(default, alias = "hotkey")]
    pub hotkey: Option<String>,
}

#[derive(Deserialize, Clone, Debug)]
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

#[derive(Deserialize, Clone, Debug)]
pub struct PageConfig {
    #[allow(dead_code)]
    pub id: String,
    pub title: String,
    pub icon: Option<String>,
    #[serde(default)]
    pub panels: Vec<PanelConfig>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct PanelConfig {
    pub id: String,
    pub title: String,
    #[serde(flatten)]
    pub kind: PanelKind,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum PanelKind {
    Stat {
        #[serde(default)]
        value: Option<f64>,
        unit: Option<String>,
        percent: Option<f32>,
    },
    Progress {
        #[serde(default)]
        value: Option<f64>,
        max: Option<f64>,
    },
    LineChart {
        data: Vec<DataPoint>,
    },
    AreaChart {
        data: Vec<DataPoint>,
    },
    BarChart {
        data: Vec<DataPoint>,
    },
    PieChart {
        data: Vec<DataPoint>,
    },
}

#[derive(Deserialize, Clone, Debug)]
pub struct DataPoint {
    pub label: String,
    pub value: f64,
}

pub fn config_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(home).join(".config/gpui-dashboard/dashboard.config.ts")
}

pub fn load_config() -> DashboardConfig {
    let path = config_path();
    match crate::js_runtime::run_config(&path) {
        Ok(value) => serde_json::from_value(value).unwrap_or_else(|e| {
            eprintln!("config deserialize failed: {e}");
            DashboardConfig::default()
        }),
        Err(e) => {
            eprintln!("load config failed: {e}");
            DashboardConfig::default()
        }
    }
}

pub fn parse_icon(name: &str) -> gpui_component::IconName {
    match name {
        "layout-dashboard" => gpui_component::IconName::LayoutDashboard,
        "gallery-vertical-end" => gpui_component::IconName::GalleryVerticalEnd,
        "chart-pie" => gpui_component::IconName::ChartPie,
        "bot" => gpui_component::IconName::Bot,
        "cpu" => gpui_component::IconName::Cpu,
        "settings" => gpui_component::IconName::Settings,
        "inbox" => gpui_component::IconName::Inbox,
        "calendar" => gpui_component::IconName::Calendar,
        "folder" => gpui_component::IconName::Folder,
        "search" => gpui_component::IconName::Search,
        "chevron-down" => gpui_component::IconName::ChevronDown,
        "chevron-right" => gpui_component::IconName::ChevronRight,
        "close" => gpui_component::IconName::Close,
        "info" => gpui_component::IconName::Info,
        "circle-check" => gpui_component::IconName::CircleCheck,
        "triangle-alert" => gpui_component::IconName::TriangleAlert,
        "circle-x" => gpui_component::IconName::CircleX,
        "user" => gpui_component::IconName::User,
        "github" => gpui_component::IconName::Github,
        "arrow-left" => gpui_component::IconName::ArrowLeft,
        "arrow-right" => gpui_component::IconName::ArrowRight,
        "minimize" => gpui_component::IconName::Minimize,
        "maximize" => gpui_component::IconName::Maximize,
        "window-minimize" => gpui_component::IconName::WindowMinimize,
        "window-maximize" => gpui_component::IconName::WindowMaximize,
        "window-close" => gpui_component::IconName::WindowClose,
        "window-restore" => gpui_component::IconName::WindowRestore,
        "panel-left" => gpui_component::IconName::PanelLeft,
        "panel-right" => gpui_component::IconName::PanelRight,
        "panel-bottom" => gpui_component::IconName::PanelBottom,
        "ellipsis" => gpui_component::IconName::Ellipsis,
        "loader" => gpui_component::IconName::Loader,
        "star" => gpui_component::IconName::Star,
        "star-fill" => gpui_component::IconName::StarFill,
        "plus" => gpui_component::IconName::Plus,
        "minus" => gpui_component::IconName::Minus,
        "check" => gpui_component::IconName::Check,
        "copy" => gpui_component::IconName::Copy,
        "eye" => gpui_component::IconName::Eye,
        "eye-off" => gpui_component::IconName::EyeOff,
        "asterisk" => gpui_component::IconName::Asterisk,
        "resize-corner" => gpui_component::IconName::ResizeCorner,
        "panel-left-open" => gpui_component::IconName::PanelLeftOpen,
        "panel-right-open" => gpui_component::IconName::PanelRightOpen,
        "panel-left-close" => gpui_component::IconName::PanelLeftClose,
        "panel-right-close" => gpui_component::IconName::PanelRightClose,
        "inspector" => gpui_component::IconName::Inspector,
        "sort-ascending" => gpui_component::IconName::SortAscending,
        "sort-descending" => gpui_component::IconName::SortDescending,
        "chevrons-up-down" => gpui_component::IconName::ChevronsUpDown,
        "undo-2" => gpui_component::IconName::Undo2,
        "case-sensitive" => gpui_component::IconName::CaseSensitive,
        "replace" => gpui_component::IconName::Replace,
        "chevron-left" => gpui_component::IconName::ChevronLeft,
        "external-link" => gpui_component::IconName::ExternalLink,
        _ => gpui_component::IconName::LayoutDashboard,
    }
}
