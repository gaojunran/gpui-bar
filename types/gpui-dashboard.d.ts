/**
 * gpui-bar 配置类型定义
 *
 * 放置在用户配置目录（~/.config/gpui-bar/），由同目录 tsconfig.json 自动 include。
 * 用户在 bar.config.ts 中无需 import，类型与 host function 全局可用。
 *
 * 注意：配置文件运行在 QuickJS（嵌入式 JS 引擎）中，不是 Node.js。
 * 不支持任何 Node API（如 require / fs / process / Buffer）。
 * 常用能力（HTTP 请求、执行命令、读环境变量等）已由 Rust 层注册为全局 JS 函数，见下方 declare。
 */

interface BarStatItem {
  label: string;
  value: number;
  unit?: string;
  /** 数值前缀,如 "¥"。渲染为小字 + 非强调色,与数值基线对齐。 */
  prefix?: string;
  /** 数值后缀,如 "%"/"次"。渲染为小字 + 非强调色。 */
  suffix?: string;
  /** 小数位数。省略时整数显示 0 位、小数显示 1 位。 */
  decimals?: number;
  color?: string;
  font?: string;
  action?: BarAction;
}

interface BarInfoLineItem {
  title: string;
  desc?: string;
  color?: string;
  descColor?: string;
  font?: string;
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
    }
  | {
      kind: "info-line";
      title?: string;
      items: BarInfoLineItem[];
    }
  | {
      kind: "info-block";
      title: string;
      desc?: string;
      color?: string;
      descColor?: string;
      font?: string;
      action?: BarAction;
    };

interface BarConfig {
  panels: BarPanel[];
}

interface Config {
  bar: BarConfig;
  /** 启动时是否置顶(浮在其它应用窗口之上),默认 true */
  alwaysOnTop?: boolean;
  /** 窗口出现在哪个显示器(0=主显示器,1=第二个...),默认 0 */
  displayIndex?: number;
  /** 唤起/隐藏 bar 的全局热键,如 "cmd+shift+b",默认 "cmd+shift+b" */
  hotkey?: string;
  /** 刷新配置的窗口级热键(仅 bar 窗口聚焦时生效),如 "cmd+r",默认 "cmd+r" */
  refreshHotkey?: string;
  /** 点击空白处时是否保持窗口聚焦(不失活),默认 false(正常失焦) */
  keepFocus?: boolean;
}

/**
 * host function: 发起 HTTP 请求，返回完整响应对象。
 * 同步阻塞执行（在专用线程），Promise 立即 resolve。
 *
 * @param url     请求 URL
 * @param options 请求选项
 * @returns       { status, headers, body }
 *                body 为 string；非 2xx 也会 resolve（看 status 判断成功）。
 *                redirect: "manual" 时不跟随重定向，用于识别 302 失效信号。
 */
declare function fetch(
  url: string,
  options?: RequestOptions
): Promise<HttpResponse<string>>;

/**
 * host function: 同 fetch，但 body 尝试 JSON.parse。
 * parse 失败时 body 回退为 string（不 reject），方便调用者判断
 * 鉴权失败返回的 HTML 登录页等场景。
 */
declare function fetchJson(
  url: string,
  options?: RequestOptions
): Promise<HttpResponse<unknown>>;

interface RequestOptions {
  method?: string;
  headers?: Record<string, string>;
  body?: string;
  /** "manual" 时不跟随重定向，用于识别 302 失效信号。默认 "follow" */
  redirect?: "manual" | "follow";
}

interface HttpResponse<T = string> {
  status: number;
  /** header 名全部小写 */
  headers: Record<string, string>;
  body: T;
}

/**
 * host function: 通过 `sh -c` 执行 shell 命令，返回 stdout。
 * 同步阻塞执行（在专用线程），命令失败时 Promise reject。
 * 注意：慢命令会拖慢 config 加载；复杂场景建议用 fetchJson 调本地 HTTP 服务。
 *
 * @param command shell 命令
 * @param options 选项。cwd 指定工作目录，默认为 $HOME
 *                （macOS GUI app 经 launchd 启动时 cwd 为 `/`，会导致 fnox 等依赖
 *                 cwd 查找配置的 CLI 工具失败，故默认回退到 HOME）。
 */
declare function exec(
  command: string,
  options?: { cwd?: string }
): Promise<string>;

/**
 * host function: 从浏览器读取 cookies。
 *
 * @param domain  可选域名过滤。注意 rookie 内部用 `LIKE '%domain%'`，
 *                可能匹配到子串（如 "woa.com" 会匹配 "xxwoa.comyy"），
 *                调用者需自行精确校验 domain 字段。
 * @param browser 浏览器名："chrome" | "firefox" | "edge" | "brave" |
 *                "arc" | "safari" | "all"（默认 "all"，尝试所有已安装浏览器）
 * @returns       cookie 数组
 *
 * macOS 首次调用会弹 Keychain 授权框（"Chrome Safe Storage wants to be accessed"）。
 */
declare function cookies(domain?: string, browser?: string): Promise<Cookie[]>;

/**
 * host function: 读取进程环境变量。
 * 变量不存在时 throw,调用方需 try/catch。
 * 同步返回,不走 Promise。
 */
declare function env(name: string): string;

/**
 * host function: 写一行日志到统一日志文件
 * (~/.config/gpui-bar/gpui-bar.log)。
 * GUI app 无 stdout,这是配置侧唯一可见的诊断通道。
 * 同步返回,不走 Promise。
 */
declare function log(msg: string): void;

interface Cookie {
  name: string;
  value: string;
  domain: string;
  path: string;
  secure: boolean;
  http_only: boolean;
  expires: number | null;
  same_site: number;
}
