# gpui-bar

A cross-platform menu bar style dashboard built with [GPUI](https://github.com/zed-industries/zed) (Zed's GPU-accelerated UI framework).

![mode: bar](https://img.shields.io/badge/mode-bar%20%2F%20dashboard-blue)
![platform: cross-platform](https://img.shields.io/badge/platform-cross--platform-blue)
![tested: macOS](https://img.shields.io/badge/tested-macOS-black)
![language: rust](https://img.shields.io/badge/language-rust-orange)

> **Cross-platform, macOS-tested.** gpui-bar aims to run anywhere GPUI does. It is currently only tested on macOS — feedback on other platforms is welcome.

## What it is

`gpui-bar` renders a compact floating card at the top-right corner of your screen, showing live stats and progress bars driven by a TypeScript config file. Click any item to open a URL, run a shell command, or invoke a custom async function. Toggle visibility with a global hotkey.

It also ships a full dashboard mode (sidebar + multi-page + charts) for when you want the same data in a regular window.

## Why GPUI

- **CSS-like styling** — lay out components with a familiar `flex` / `gap` / `padding` / `color` API, no widget boilerplate.
- **Extreme efficiency** — GPU-accelerated rendering keeps CPU, binary size, and memory footprint minimal.
- **Zero-IDE lock-in** — no Xcode or platform-specific project files. `cargo run` and you're done.

## Features

- **Two templates** for the bar
  - `stat-row` — a row of labeled numbers, each independently colored and clickable
  - `progress-bar` — a labeled progress bar with value / max
- **TypeScript config** — write your config in TS, transpiled and evaluated in an embedded QuickJS runtime. Use `fetch` / `fetchJson` to pull live data from any HTTP API.
- **Per-item styling** — hex color and font family per stat item
- **Click actions** — `url` (open in browser), `command` (shell via `sh -c`), or `function` (call an exported TS function, sync or async)
- **Multi-monitor** — pick which display the bar appears on via `displayIndex`
- **Global hotkey** — toggle the bar from anywhere (default `cmd+shift+b`)
- **Refresh hotkey** — reload the config on demand without restarting (default `cmd+r`, bar mode only)
- **Always-on-top** — float above other apps via `WindowKind::Floating` (default on)
- **Auto-refresh** — config is re-evaluated on an interval so values stay live
- **Dashboard mode** — if no `bar` is configured, a full window opens with a sidebar, pages, and chart panels (stat / progress / line / area / bar / pie)

## Installation

### From source

Requires Rust (stable). On macOS, GPUI's Metal shaders are compiled at runtime via the `runtime_shaders` feature, so a full Xcode install is **not** required — `cargo run` works out of the box.

```bash
git clone https://github.com/gaojunran/gpui-bar.git
cd gpui-bar
cargo run --release
```

### Configuration

The config file lives at `~/.config/gpui-bar/bar.config.ts`. TypeScript type definitions are shipped in `types/gpui-dashboard.d.ts` — copy them next to your config for autocomplete (see `scripts/install-types.sh`).

## Config reference

```ts
export default async function getConfig(): Promise<DashboardConfig> {
  // Fetch live data here if you want — async is supported.
  const repo = await fetchJson("https://api.github.com/repos/gaojunran/gpui-bar");
  const stars: number = repo.stargazers_count;

  return {
    title: "Dashboard",
    refreshInterval: 300,          // seconds between re-evaluations
    displayIndex: 0,               // which monitor (0 = primary)
    alwaysOnTop: true,             // float above other apps
    hotkey: "cmd+shift+b",         // global toggle hotkey
    refreshHotkey: "cmd+r",        // reload config (bar mode only)

    bar: {
      panels: [
        {
          kind: "stat-row",
          items: [
            { label: "Stars", value: stars, color: "#fbbf24",
              action: { type: "url", url: "https://github.com/gaojunran/gpui-bar" } },
            { label: "Forks", value: 4, color: "#60a5fa",
              action: { type: "command", command: "echo hi >> /tmp/bar.log" } },
            { label: "Issues", value: 2, color: "#f87171",
              action: { type: "function", name: "onIssuesClick" } },
          ],
        },
        {
          kind: "progress-bar",
          label: "Quota",
          value: 7.2, max: 10, unit: "GB",
          color: "#34d399",
          action: { type: "url", url: "https://example.com/billing" },
        },
      ],
    },
  };
}

// Custom functions can be sync or async; use fetch / fetchJson inside.
export async function onIssuesClick() {
  await fetch("https://httpbin.org/get");
}
```

### Config fields

| Field | Type | Default | Description |
|---|---|---|---|
| `title` | `string?` | — | App / window title |
| `refreshInterval` | `number?` | `60` | Seconds between config re-evaluations |
| `displayIndex` | `number?` | `0` | Which monitor to place the bar on |
| `alwaysOnTop` | `boolean?` | `true` | Float above other app windows |
| `hotkey` | `string?` | `"cmd+shift+b"` | Global hotkey to toggle the bar |
| `refreshHotkey` | `string?` | `"cmd+r"` | Window-level hotkey to reload the config (bar mode only) |
| `bar` | `BarConfig?` | — | If present, runs in bar mode. If absent, runs in dashboard mode |

### Bar panel kinds

- **`stat-row`** — `items: BarStatItem[]`
- **`progress-bar`** — `label`, `value`, `max`, `unit?`, `color?`, `font?`, `action?`

### Action types

- `{ type: "url", url: string }` — open in the default browser
- `{ type: "command", command: string }` — run via `sh -c`
- `{ type: "function", name: string }` — call an exported TS function

### Hotkey syntax

Modifiers joined by `+`, followed by a key. Modifiers: `cmd` / `super` / `meta`, `ctrl`, `alt` / `option`, `shift`. Keys: `a`-`z`, `0`-`9`, `f1`-`f12`, `up`/`down`/`left`/`right`, `space`, `enter`, `tab`, `esc`, `backspace`, `delete`.

Examples: `"cmd+shift+b"`, `"ctrl+alt+d"`, `"super+f1"`.

## How it works

- **GPUI** handles the GPU-accelerated rendering and window management. The bar window uses `WindowKind::Floating` (NSFloatingWindowLevel) for always-on-top, and `titlebar: None` for a borderless card.
- **rquickjs** + **oxc** transpile and run your TypeScript config in an embedded QuickJS runtime. `fetch` / `fetchJson` are host functions backed by `reqwest::blocking`.
- **global-hotkey** registers a Carbon `RegisterEventHotKey` for the toggle, polled from a GPUI background task. No extra macOS permissions are required.
- The config is re-evaluated on `refreshInterval`, so values update live without restarting.

## Project layout

```
src/
  main.rs         # entry point, window creation, hotkey loop
  bar.rs          # bar mode rendering + click actions
  dashboard.rs    # dashboard mode (sidebar + pages)
  panel.rs        # chart panels for dashboard mode
  config.rs       # config schema + serde
  js_runtime.rs   # TS -> JS transpile + QuickJS eval
  hotkey.rs       # hotkey string parser
types/
  gpui-dashboard.d.ts   # TS types for user configs
scripts/
  install-types.sh      # scaffold ~/.config/gpui-bar
```

## License

MIT
