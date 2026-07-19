use anyhow::Context as _;
use rquickjs::{Context, Ctx, Function, Module, Promise, Runtime};
use serde_json::Value;
use std::path::Path;

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

/// host function: fetch(url) -> Promise<string>
/// 同步阻塞 reqwest::blocking，在 JS 线程内完成，Promise 立即 resolve。
fn fetch_handler<'js>(ctx: Ctx<'js>, url: String) -> rquickjs::Result<Promise<'js>> {
    let (promise, resolve, reject) = Promise::new(&ctx)?;
    let result = http_client().get(&url).send().and_then(|r| r.text());
    match result {
        Ok(body) => resolve.call::<_, ()>((body,))?,
        Err(e) => {
            eprintln!("[fetch] error for {url}: {e}");
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
                eprintln!("[exec] `{command}` failed (status {:?}): {stderr}", out.status.code());
            }
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            resolve.call::<_, ()>((stdout,))?;
        }
        Err(e) => {
            eprintln!("[exec] `{command}` spawn failed: {e}");
            reject.call::<_, ()>((e.to_string(),))?;
        }
    }
    Ok(promise)
}

/// host function: fetchJson(url) -> Promise<any>
/// fetch 后在 JS 侧 JSON.parse，避免 Rust↔JS 值转换。
fn fetch_json_handler<'js>(ctx: Ctx<'js>, url: String) -> rquickjs::Result<Promise<'js>> {
    let (promise, resolve, reject) = Promise::new(&ctx)?;
    let result = http_client().get(&url).send().and_then(|r| r.text());
    match result {
        Ok(body) => {
            ctx.globals().set("__fetch_json_body", body)?;
            // JSON.parse 失败时 reject 而非让 host function 抛异常
            match ctx.eval::<rquickjs::Value, _>("JSON.parse(__fetch_json_body)") {
                Ok(parsed) => resolve.call::<_, ()>((parsed,))?,
                Err(_) => {
                    let msg = "fetchJson: response is not valid JSON";
                    reject.call::<_, ()>((msg,))?;
                }
            }
        }
        Err(e) => {
            eprintln!("[fetchJson] error for {url}: {e}");
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
