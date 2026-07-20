use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyManager,
};

/// 解析用户配置的热键字符串为 global-hotkey 的 HotKey。
/// 支持格式: "cmd+shift+b", "ctrl+alt+d", "super+f1" 等。
/// modifier 别名: cmd/super/meta -> SUPER, ctrl/control -> CONTROL, alt/option -> ALT, shift -> SHIFT
pub fn parse_hotkey(s: &str) -> anyhow::Result<HotKey> {
    let lower = s.trim().to_lowercase();
    let parts: Vec<&str> = lower.split('+').collect();
    if parts.is_empty() {
        anyhow::bail!("hotkey 不能为空");
    }

    let mut mods = Modifiers::empty();
    let mut key_str: Option<&str> = None;

    for part in &parts {
        let p = part.trim();
        match p {
            "cmd" | "super" | "meta" | "win" => mods |= Modifiers::SUPER,
            "ctrl" | "control" => mods |= Modifiers::CONTROL,
            "alt" | "option" | "opt" => mods |= Modifiers::ALT,
            "shift" => mods |= Modifiers::SHIFT,
            _ => {
                if key_str.is_some() {
                    anyhow::bail!("热键只能有一个主键,发现多个: {s}");
                }
                key_str = Some(p);
            }
        }
    }

    let key_str = key_str.ok_or_else(|| anyhow::anyhow!("热键缺少主键: {s}"))?;
    let code = parse_key_code(key_str)?;

    Ok(HotKey::new(Some(mods), code))
}

/// 将用户友好的键名转换为 keyboard-types::Code
fn parse_key_code(s: &str) -> anyhow::Result<Code> {
    let code = match s {
        // 字母键
        "a" => Code::KeyA, "b" => Code::KeyB, "c" => Code::KeyC, "d" => Code::KeyD,
        "e" => Code::KeyE, "f" => Code::KeyF, "g" => Code::KeyG, "h" => Code::KeyH,
        "i" => Code::KeyI, "j" => Code::KeyJ, "k" => Code::KeyK, "l" => Code::KeyL,
        "m" => Code::KeyM, "n" => Code::KeyN, "o" => Code::KeyO, "p" => Code::KeyP,
        "q" => Code::KeyQ, "r" => Code::KeyR, "s" => Code::KeyS, "t" => Code::KeyT,
        "u" => Code::KeyU, "v" => Code::KeyV, "w" => Code::KeyW, "x" => Code::KeyX,
        "y" => Code::KeyY, "z" => Code::KeyZ,
        // 数字键
        "0" => Code::Digit0, "1" => Code::Digit1, "2" => Code::Digit2, "3" => Code::Digit3,
        "4" => Code::Digit4, "5" => Code::Digit5, "6" => Code::Digit6, "7" => Code::Digit7,
        "8" => Code::Digit8, "9" => Code::Digit9,
        // 功能键
        "f1" => Code::F1, "f2" => Code::F2, "f3" => Code::F3, "f4" => Code::F4,
        "f5" => Code::F5, "f6" => Code::F6, "f7" => Code::F7, "f8" => Code::F8,
        "f9" => Code::F9, "f10" => Code::F10, "f11" => Code::F11, "f12" => Code::F12,
        // 方向键
        "up" | "arrowup" => Code::ArrowUp,
        "down" | "arrowdown" => Code::ArrowDown,
        "left" | "arrowleft" => Code::ArrowLeft,
        "right" | "arrowright" => Code::ArrowRight,
        // 特殊键
        "space" => Code::Space,
        "enter" | "return" => Code::Enter,
        "tab" => Code::Tab,
        "esc" | "escape" => Code::Escape,
        "backspace" => Code::Backspace,
        "delete" | "del" => Code::Delete,
        _ => anyhow::bail!("不支持的键: {s}"),
    };
    Ok(code)
}

/// 注册全局热键,返回 (热键id, manager)。manager 必须保持存活,drop 会注销热键。
pub fn register(hotkey_str: &str) -> anyhow::Result<(u32, GlobalHotKeyManager)> {
    let hotkey = parse_hotkey(hotkey_str)?;
    let id = hotkey.id();
    let manager = GlobalHotKeyManager::new()?;
    manager.register(hotkey)?;
    Ok((id, manager))
}

/// 将用户配置的热键字符串(plus 格式,如 "cmd+r"、"cmd+shift+b")转换为
/// GPUI KeyBinding 使用的 hyphen 格式(如 "cmd-r"、"cmd-shift-b"),
/// 并把 global-hotkey 接受的修饰键别名规范化为 GPUI 的名称
/// (control→ctrl, option/opt→alt, meta→cmd;cmd/super/win/ctrl/alt/shift 原样保留)。
/// 主键原样透传,合法性交由 GPUI 的 Keystroke::parse 在调用处校验。
pub fn to_gpui_keystroke(s: &str) -> anyhow::Result<String> {
    let lower = s.trim().to_lowercase();
    if lower.is_empty() {
        anyhow::bail!("hotkey 不能为空");
    }
    let parts: Vec<&str> = lower
        .split('+')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();
    if parts.len() < 2 {
        anyhow::bail!("热键至少需要修饰键+主键: {s}");
    }
    let (mods, key) = parts.split_at(parts.len() - 1);
    let mut tokens: Vec<String> = mods
        .iter()
        .map(|m| match *m {
            "control" => "ctrl".into(),
            "option" | "opt" => "alt".into(),
            "meta" => "cmd".into(),
            other => other.into(),
        })
        .collect();
    tokens.push(key[0].into());
    Ok(tokens.join("-"))
}
