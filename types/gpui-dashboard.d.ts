/**
 * gpui-dashboard 配置类型定义
 *
 * 放置在用户配置目录（~/.config/gpui-dashboard/），由同目录 tsconfig.json 自动 include。
 * 用户在 dashboard.config.ts 中无需 import，类型与 host function 全局可用。
 */

interface DataPoint {
  label: string;
  value: number;
  color?: string;
}

interface BarStatItem {
  label: string;
  value: number;
  unit?: string;
  color?: string;  // hex 如 "#ff5500"
  font?: string;   // 字体名 如 "Helvetica"
  action?: BarAction;
}

/** 点击动作 */
type BarAction =
  | { type: "url"; url: string }
  | { type: "command"; command: string }
  | { type: "function"; name: string };

type BarPanel =
  | {
      kind: "stat-row";
      items: BarStatItem[];
    }
  | {
      kind: "progress-bar";
      label: string;
      value: number;
      max: number;
      unit?: string;
      color?: string;
      font?: string;
      action?: BarAction;
    };

interface BarConfig {
  panels: BarPanel[];
}

type PanelConfig =
  | {
      id: string;
      title: string;
      kind: "stat";
      value?: number;
      unit?: string;
      percent?: number;
    }
  | {
      id: string;
      title: string;
      kind: "progress";
      value?: number;
      max?: number;
    }
  | {
      id: string;
      title: string;
      kind: "line-chart";
      data: DataPoint[];
    }
  | {
      id: string;
      title: string;
      kind: "area-chart";
      data: DataPoint[];
    }
  | {
      id: string;
      title: string;
      kind: "bar-chart";
      data: DataPoint[];
    }
  | {
      id: string;
      title: string;
      kind: "pie-chart";
      data: DataPoint[];
    };

interface PageConfig {
  id: string;
  title: string;
  icon?: string;
  panels: PanelConfig[];
}

interface DashboardConfig {
  title?: string;
  refreshInterval?: number;
  pages?: PageConfig[];
  bar?: BarConfig;
  /** 启动时是否置顶(浮在其它应用窗口之上),默认 true */
  alwaysOnTop?: boolean;
  /** 窗口出现在哪个显示器(0=主显示器,1=第二个...),默认 0 */
  displayIndex?: number;
  /** 唤起/隐藏 bar 的全局热键,如 "cmd+shift+b",默认 "cmd+shift+b" */
  hotkey?: string;
}

/** 支持的图标名（与 Rust 端 parse_icon 匹配，未列出的回退到 layout-dashboard） */
type IconName =
  | "layout-dashboard"
  | "gallery-vertical-end"
  | "chart-pie"
  | "bot"
  | "cpu"
  | "settings"
  | "inbox"
  | "calendar"
  | "folder"
  | "search"
  | "star"
  | "github"
  | "user";

/**
 * host function: HTTP GET，返回响应体文本。
 * 同步阻塞执行（在专用线程），Promise 立即 resolve。
 */
declare function fetch(url: string): Promise<string>;

/**
 * host function: HTTP GET + JSON.parse，返回已解析的对象。
 * 推荐：响应非合法 JSON 时 Promise reject。
 */
declare function fetchJson(url: string): Promise<unknown>;

/**
 * host function: 通过 `sh -c` 执行 shell 命令，返回 stdout。
 * 同步阻塞执行（在专用线程），命令失败时 Promise reject。
 * 注意：慢命令会拖慢 config 加载；复杂场景建议用 fetchJson 调本地 HTTP 服务。
 */
declare function exec(command: string): Promise<string>;
