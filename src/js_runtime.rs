use anyhow::Context as _;
use rquickjs::{Context, Ctx, Function, Module, Promise, Runtime};
use serde_json::Value;
use std::io::Write;
use std::path::Path;

/// 统一日志文件路径:`~/.config/gpui-dashboard/gpui-bar.log`
/// GUI app 由 launchd 拉起,stdout/stderr 不可见,所有诊断日志落盘到这里。
fn log_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    std::path::PathBuf::from(home).join(".config/gpui-dashboard/gpui-bar.log")
}

/// 写一行带时间戳的日志到 log_path()。失败静默(日志不应影响主流程)。
pub(crate) fn write_log(tag: &str, msg: &str) {
    let path = log_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let ts = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    let line = format!("[{ts}] {tag} {msg}\n");
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        let _ = f.write_all(line.as_bytes());
    }
}

/// host function: log(...args) -> undefined
/// 把任意数量参数转成字符串(对象经 JSON.stringify)写入统一日志文件。
/// 供配置侧打点排查;GUI app 无 stdout,这是用户侧唯一可见的诊断通道。
fn log_handler<'js>(ctx: Ctx<'js>, args: Vec<rquickjs::Value<'js>>) -> rquickjs::Result<()> {
    let mut parts: Vec<String> = Vec::with_capacity(args.len());
    for v in args {
        if v.is_undefined() {
            parts.push("undefined".into());
        } else if v.is_null() {
            parts.push("null".into());
        } else if let Some(s) = v.as_string().and_then(|s| s.to_string().ok()) {
            parts.push(s);
        } else {
            ctx.globals().set("__log_arg", v.clone())?;
            let s: String = ctx.eval::<String, _>("JSON.stringify(__log_arg)")?;
            parts.push(s);
        }
    }
    write_log("[config]", &parts.join(" "));
    Ok(())
}

/// 将 TypeScript 源码剥离类型注解，输出纯 JS。
/// 链路：Parser → SemanticBuilder → Transformer(strip types) → Codegen
fn strip_typescript(ts: &str) -> anyhow::Result<String> {
    use oxc_allocator::Allocator;
    use oxc_codegen::Codegen;
    use oxc_parser::Parser;
    use oxc_semantic::SemanticBuilder;
    use oxc_span::SourceType;
    use oxc_transformer::Transformer;

    let allocator = Allocator::default();
    let source_type = SourceType::from_path("config.ts").unwrap_or_default();

    let parser_ret = Parser::new(&allocator, ts, source_type).parse();
    if !parser_ret.diagnostics.is_empty() {
        anyhow::bail!("oxc parse errors: {:?}", parser_ret.diagnostics);
    }
    let mut program = parser_ret.program;

    let semantic_ret = SemanticBuilder::new().build(&program);
    if !semantic_ret.diagnostics.is_empty() {
        anyhow::bail!("oxc semantic errors: {:?}", semantic_ret.diagnostics);
    }
    let scoping = semantic_ret.semantic.into_scoping();

    let options = oxc_transformer::TransformOptions::default();
    let transformer_ret =
        Transformer::new(&allocator, std::path::Path::new("config.ts"), &options)
            .build_with_scoping(scoping, &mut program);
    if !transformer_ret.diagnostics.is_empty() {
        anyhow::bail!("oxc transform errors: {:?}", transformer_ret.diagnostics);
    }

    Ok(Codegen::new().build(&program).code)
}

/// 构建带 User-Agent 的 blocking client（GitHub 等 API 要求 UA）。
fn http_client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .user_agent("gpui-dashboard/0.1")
        .build()
        .expect("build reqwest client")
}

/// 不自动跟随重定向的 client(用于识别 302 passport.woa.com 等失效信号)
fn http_client_no_redirect() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .user_agent("gpui-dashboard/0.1")
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("build reqwest client")
}

/// fetch / fetchJson 的请求选项,从 JS options 对象经 JSON 桥接反序列化。
#[derive(serde::Deserialize, Default)]
struct RequestOptions {
    #[serde(default)]
    method: Option<String>,
    #[serde(default)]
    headers: Option<std::collections::HashMap<String, String>>,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    redirect: Option<String>,
}

/// fetch / fetchJson 的响应体。body 用 serde_json::Value,
/// fetch 时为 string,fetchJson 时为已解析的 JSON 值(parse 失败回退为 string)。
#[derive(serde::Serialize)]
struct HttpResponse {
    status: u16,
    headers: std::collections::HashMap<String, String>,
    body: serde_json::Value,
}

/// 把 JS 侧传入的 options Value 经 JSON.stringify -> serde_json 反序列化为 RequestOptions。
/// undefined / null 返回默认值(全 GET、无头、follow redirect)。
fn parse_options<'js>(
    ctx: &Ctx<'js>,
    options: Option<rquickjs::Value<'js>>,
) -> rquickjs::Result<RequestOptions> {
    match options {
        Some(v) if !v.is_undefined() && !v.is_null() => {
            ctx.globals().set("__req_opts", v)?;
            let json: String = ctx.eval::<String, _>("JSON.stringify(__req_opts)")?;
            Ok(serde_json::from_str(&json).unwrap_or_default())
        }
        _ => Ok(RequestOptions::default()),
    }
}

/// 执行一次 HTTP 请求,返回 reqwest::blocking::Response。
/// 网络错误重试最多 3 次(间隔 500ms),避免偶发连接失败。
fn do_request(url: &str, opts: &RequestOptions) -> anyhow::Result<reqwest::blocking::Response> {
    let client = if opts.redirect.as_deref() == Some("manual") {
        http_client_no_redirect()
    } else {
        http_client()
    };
    let method = opts.method.as_deref().unwrap_or("GET");
    let m = reqwest::Method::from_bytes(method.as_bytes())
        .map_err(|e| anyhow::anyhow!("invalid method `{method}`: {e}"))?;

    let mut last_err: Option<anyhow::Error> = None;
    for attempt in 0..3u32 {
        let mut builder = client.request(m.clone(), url);
        if let Some(h) = &opts.headers {
            for (k, v) in h {
                if let Ok(name) = reqwest::header::HeaderName::from_bytes(k.as_bytes()) {
                    if let Ok(val) = reqwest::header::HeaderValue::from_str(v) {
                        builder = builder.header(name, val);
                    }
                }
            }
        }
        if let Some(body) = &opts.body {
            builder = builder.body(body.clone());
        }
        match builder.send() {
            Ok(resp) => return Ok(resp),
            Err(e) => {
                last_err = Some(anyhow::anyhow!(e));
                if attempt < 2 {
                    std::thread::sleep(std::time::Duration::from_millis(500));
                }
            }
        }
    }
    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("request failed after 3 attempts")))
}

/// 把 reqwest Response 转成 JS HttpResponse 对象并 resolve。
/// parse_json=true 时尝试把 body 解析成 JSON(失败回退为 string,不 reject,
/// 这样鉴权失败返回 HTML 登录页时调用者能拿到原始 body 判断)。
fn resolve_response<'js>(
    ctx: &Ctx<'js>,
    resolve: &rquickjs::Function<'js>,
    reject: &rquickjs::Function<'js>,
    resp: reqwest::blocking::Response,
    parse_json: bool,
) -> rquickjs::Result<()> {
    let status = resp.status().as_u16();
    let mut hdrs: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for (k, v) in resp.headers() {
        if let Ok(s) = v.to_str() {
            hdrs.insert(k.as_str().to_lowercase(), s.to_string());
        }
    }
    let body_text = resp.text().unwrap_or_default();
    let body_value: serde_json::Value = if parse_json {
        serde_json::from_str(&body_text).unwrap_or(serde_json::Value::String(body_text))
    } else {
        serde_json::Value::String(body_text)
    };
    let http_resp = HttpResponse {
        status,
        headers: hdrs,
        body: body_value,
    };
    match serde_json::to_string(&http_resp) {
        Ok(json) => {
            ctx.globals().set("__resp_json", json)?;
            match ctx.eval::<rquickjs::Value, _>("JSON.parse(__resp_json)") {
                Ok(v) => resolve.call::<_, ()>((v,)),
                Err(e) => reject.call::<_, ()>((e.to_string(),)),
            }
        }
        Err(e) => reject.call::<_, ()>((e.to_string(),)),
    }
}

/// host function: fetch(url, options?) -> Promise<HttpResponse>
/// options: { method?, headers?, body?, redirect?: "manual" | "follow" }
/// 返回 { status, headers, body },body 为 string。
fn fetch_handler<'js>(
    ctx: Ctx<'js>,
    url: String,
    options: Option<rquickjs::Value<'js>>,
) -> rquickjs::Result<Promise<'js>> {
    let (promise, resolve, reject) = Promise::new(&ctx)?;
    let opts = parse_options(&ctx, options)?;
    match do_request(&url, &opts) {
        Ok(resp) => resolve_response(&ctx, &resolve, &reject, resp, false)?,
        Err(e) => {
            write_log("[fetch]", &format!("error for {url}: {e}"));
            reject.call::<_, ()>((e.to_string(),))?;
        }
    }
    Ok(promise)
}

/// host function: exec(command) -> Promise<string>
/// 同步执行 shell 命令(sh -c),返回 stdout。失败时 reject,stderr 打到 stderr。
/// 实际是串行阻塞(和 fetch 一致),慢命令会拖慢 config 加载。
fn exec_handler<'js>(ctx: Ctx<'js>, command: String) -> rquickjs::Result<Promise<'js>> {
    let (promise, resolve, reject) = Promise::new(&ctx)?;
    match std::process::Command::new("sh")
        .arg("-c")
        .arg(&command)
        .output()
    {
        Ok(out) => {
            if !out.status.success() {
                let stderr = String::from_utf8_lossy(&out.stderr);
                write_log("[exec]", &format!("`{command}` failed (status {:?}): {stderr}", out.status.code()));
            }
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            resolve.call::<_, ()>((stdout,))?;
        }
        Err(e) => {
            write_log("[exec]", &format!("`{command}` spawn failed: {e}"));
            reject.call::<_, ()>((e.to_string(),))?;
        }
    }
    Ok(promise)
}

/// 从浏览器读取 cookie,序列化为 JSON 供 JS 侧 JSON.parse。
#[derive(serde::Serialize)]
struct CookieOut {
    name: String,
    value: String,
    domain: String,
    path: String,
    secure: bool,
    http_only: bool,
    expires: Option<u64>,
    same_site: i64,
}

impl From<rookie::enums::Cookie> for CookieOut {
    fn from(c: rookie::enums::Cookie) -> Self {
        Self {
            name: c.name,
            value: c.value,
            domain: c.domain,
            path: c.path,
            secure: c.secure,
            http_only: c.http_only,
            expires: c.expires,
            same_site: c.same_site,
        }
    }
}

/// host function: env(name) -> string
/// 读取进程环境变量。变量不存在时 throw JS 异常(让 TS 侧 try/catch 处理)。
/// 同步返回,不走 Promise(env 读取是纯内存操作)。
fn env_handler<'js>(ctx: Ctx<'js>, name: String) -> rquickjs::Result<String> {
    match std::env::var(&name) {
        Ok(v) => Ok(v),
        Err(e) => {
            write_log("[env]", &format!("`{name}` not found: {e}"));
            Err(rquickjs::Exception::throw_message(
                &ctx,
                &format!("env `{name}` not found: {e}"),
            ))
        }
    }
}

/// host function: cookies(domain?, browser?) -> Promise<Cookie[]>
/// domain: 可选,过滤域名(rookie 用 LIKE '%domain%',可能 false-match,调用者需自行验证)
/// browser: "chrome" | "firefox" | "edge" | "brave" | "arc" | "safari" | "all"(默认)
/// macOS 首次调用会弹 Keychain 授权框("Chrome Safe Storage wants to be accessed")
fn cookies_handler<'js>(
    ctx: Ctx<'js>,
    domain: Option<String>,
    browser: Option<String>,
) -> rquickjs::Result<Promise<'js>> {
    let (promise, resolve, reject) = Promise::new(&ctx)?;
    let domains = domain.map(|d| vec![d]);
    let result = match browser.as_deref() {
        Some("chrome") => rookie::chrome(domains),
        Some("firefox") => rookie::firefox(domains),
        Some("edge") => rookie::edge(domains),
        Some("brave") => rookie::brave(domains),
        Some("arc") => rookie::arc(domains),
        Some("safari") => rookie::safari(domains),
        Some("all") | None => rookie::load(domains),
        Some(other) => {
            let msg = format!("unknown browser: {other} (supported: chrome, firefox, edge, brave, arc, safari, all)");
            write_log("[cookies]", &msg);
            reject.call::<_, ()>((msg,))?;
            return Ok(promise);
        }
    };
    match result {
        Ok(raw_cookies) => {
            let cookies: Vec<CookieOut> = raw_cookies.into_iter().map(Into::into).collect();
            match serde_json::to_string(&cookies) {
                Ok(json) => {
                    ctx.globals().set("__cookies_json", json)?;
                    match ctx.eval::<rquickjs::Value, _>("JSON.parse(__cookies_json)") {
                        Ok(parsed) => resolve.call::<_, ()>((parsed,))?,
                        Err(e) => reject.call::<_, ()>((e.to_string(),))?,
                    }
                }
                Err(e) => reject.call::<_, ()>((e.to_string(),))?,
            }
        }
        Err(e) => {
            write_log("[cookies]", &format!("read failed: {e}"));
            reject.call::<_, ()>((e.to_string(),))?;
        }
    }
    Ok(promise)
}

/// host function: fetchJson(url, options?) -> Promise<HttpResponse>
/// 同 fetch,但 body 尝试解析为 JSON(失败回退为 string,不 reject)。
/// options 与 fetch 一致。
fn fetch_json_handler<'js>(
    ctx: Ctx<'js>,
    url: String,
    options: Option<rquickjs::Value<'js>>,
) -> rquickjs::Result<Promise<'js>> {
    let (promise, resolve, reject) = Promise::new(&ctx)?;
    let opts = parse_options(&ctx, options)?;
    match do_request(&url, &opts) {
        Ok(resp) => resolve_response(&ctx, &resolve, &reject, resp, true)?,
        Err(e) => {
            write_log("[fetchJson]", &format!("error for {url}: {e}"));
            reject.call::<_, ()>((e.to_string(),))?;
        }
    }
    Ok(promise)
}

/// 读取 .ts 配置文件，转译执行 getConfig，返回 JSON 结果。
///
/// 整个流程在调用线程内同步完成（rquickjs Runtime 非 Send，需固定线程）。
/// GPUI 侧应通过 cx.background_spawn 调用此函数。
pub fn run_config(config_path: &Path) -> anyhow::Result<Value> {
    let ts = std::fs::read_to_string(config_path)
        .with_context(|| format!("read config file: {}", config_path.display()))?;
    let js = strip_typescript(&ts)?;

    let rt = Runtime::new()?;
    let ctx = Context::full(&rt)?;
    let value = ctx.with(|ctx| -> anyhow::Result<Value> {
        let global = ctx.globals();
        global.set("fetch", Function::new(ctx.clone(), fetch_handler)?)?;
        global.set("fetchJson", Function::new(ctx.clone(), fetch_json_handler)?)?;
        global.set("exec", Function::new(ctx.clone(), exec_handler)?)?;
        global.set("cookies", Function::new(ctx.clone(), cookies_handler)?)?;
        global.set("env", Function::new(ctx.clone(), env_handler)?)?;
        global.set("log", Function::new(ctx.clone(), log_handler)?)?;

        // Module API 处理 export default
        let module = Module::declare(ctx.clone(), "config", js.as_str())?;
        let (evaluated, eval_promise) = module.eval()?;
        eval_promise.finish::<()>()?;

        let config_func: Function = evaluated.get("default")?;

        // 调用 async getConfig，finish 驱动 job queue 完成
        let call_promise: Promise = config_func.call(())?;
        let raw: rquickjs::Value = call_promise.finish()?;

        // 用 JSON.stringify 桥接 JS Value -> serde_json::Value
        ctx.globals().set("__config_result", raw)?;
        let json_str: String = ctx.eval::<String, _>("JSON.stringify(__config_result)")?;
        let value: Value = serde_json::from_str(&json_str)?;
        write_log("[config]", &format!("getConfig ok, json bytes={}", json_str.len()));
        Ok(value)
    })?;

    Ok(value)
}

/// 重新加载配置模块，调用其中的具名导出函数。
///
/// 用于 bar 点击 action 的 `function` 类型。每次调用都会新建 Runtime
/// (rquickjs Runtime 非 Send，需固定在调用线程；GPUI 侧应通过
/// `cx.background_spawn` 调用，Runtime 在该后台线程内创建并销毁)。
/// 支持同步和 async 函数(async 函数返回的 Promise 会被 finish 驱动完成)。
/// 返回值被丢弃(点击动作只关心副作用)。函数可使用 fetch/fetchJson。
pub fn call_config_function(config_path: &Path, func_name: &str) -> anyhow::Result<()> {
    let ts = std::fs::read_to_string(config_path)
        .with_context(|| format!("read config file: {}", config_path.display()))?;
    let js = strip_typescript(&ts)?;

    let rt = Runtime::new()?;
    let ctx = Context::full(&rt)?;
    ctx.with(|ctx| -> anyhow::Result<()> {
        let global = ctx.globals();
        global.set("fetch", Function::new(ctx.clone(), fetch_handler)?)?;
        global.set("fetchJson", Function::new(ctx.clone(), fetch_json_handler)?)?;
        global.set("exec", Function::new(ctx.clone(), exec_handler)?)?;
        global.set("cookies", Function::new(ctx.clone(), cookies_handler)?)?;
        global.set("env", Function::new(ctx.clone(), env_handler)?)?;
        global.set("log", Function::new(ctx.clone(), log_handler)?)?;

        let module = Module::declare(ctx.clone(), "config", js.as_str())?;
        let (evaluated, eval_promise) = module.eval()?;
        eval_promise.finish::<()>()?;

        let func: Function = evaluated.get(func_name).with_context(|| {
            format!("config 未导出名为 `{func_name}` 的函数")
        })?;

        // 调用函数；若返回 Promise 则驱动至完成
        let raw: rquickjs::Value = func.call(())?;
        if raw.is_promise() {
            if let Some(promise) = raw.into_promise() {
                let _: rquickjs::Value = promise.finish()?;
            }
        }
        Ok(())
    })?;

    Ok(())
}
