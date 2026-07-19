use rquickjs::{Context, Ctx, Function, Module, Object, Promise, Runtime};

fn main() {
    println!("=== Spike: rquickjs + oxc integration ===\n");

    test_promise_finish().unwrap_or_else(|e| println!("[1] FAILED: {e}\n"));
    test_host_function().unwrap_or_else(|e| println!("[2] FAILED: {e}\n"));
    test_oxc_strip().unwrap_or_else(|e| println!("[3] FAILED: {e}\n"));
    test_serde_bridge().unwrap_or_else(|e| println!("[4] FAILED: {e}\n"));
    test_module_export().unwrap_or_else(|e| println!("[5] FAILED: {e}\n"));
}

/// 验证点 1: rquickjs 同步 Runtime + async 函数 + Promise::finish()
fn test_promise_finish() -> anyhow::Result<()> {
    println!("--- Test 1: Promise::finish() ---");
    let rt = Runtime::new()?;
    let ctx = Context::full(&rt)?;

    ctx.with(|ctx| -> anyhow::Result<()> {
        // async 函数直接返回 JSON 字符串，方便 Rust 侧用 String 接收
        ctx.eval::<(), _>(
            r#"
            async function getConfig() {
                return JSON.stringify({ title: "test", count: 42 });
            }
            "#,
        )?;

        let global = ctx.globals();
        let func: Function = global.get("getConfig")?;
        let promise: Promise = func.call(())?;

        // finish() 循环 execute_pending_job 直到 promise resolve/reject
        let json_str: String = promise.finish()?;
        println!("[1] Promise result JSON: {json_str}");
        Ok(())
    })?;

    println!("[1] OK\n");
    Ok(())
}

// 命名函数 + 显式 'js 泛型，解决闭包 HRTB lifetime 推导问题。
// Ctx 作为第一个参数会被 rquickjs 自动注入（FromParam::param_requirement == none）。
fn fetch_handler<'js>(ctx: Ctx<'js>, url: String) -> rquickjs::Result<Promise<'js>> {
    println!("[2] host fetch called: {url}");
    let (promise, resolve, _) = Promise::new(&ctx)?;
    let resp = format!("response for {url}");
    resolve.call::<_, ()>((resp,))?;
    Ok(promise)
}

/// 验证点 2: Function::new 闭包获取 Ctx + Promise::new 注册 host function
fn test_host_function() -> anyhow::Result<()> {
    println!("--- Test 2: Host function + Promise::new ---");
    let rt = Runtime::new()?;
    let ctx = Context::full(&rt)?;

    ctx.with(|ctx| -> anyhow::Result<()> {
        let global = ctx.globals();

        // 注册 console.log
        let console = Object::new(ctx.clone())?;
        console.set(
            "log",
            Function::new(ctx.clone(), |msg: String| {
                println!("[2] console.log: {msg}");
            })?,
        )?;
        global.set("console", console)?;

        // 注册 host fetch：用命名函数避免闭包 lifetime 问题
        global.set("myFetch", Function::new(ctx.clone(), fetch_handler)?)?;

        ctx.eval::<(), _>(
            r#"
            async function testHost() {
                const r = await myFetch("http://example.com");
                console.log("got: " + r);
                return r;
            }
            "#,
        )?;

        let func: Function = global.get("testHost")?;
        let promise: Promise = func.call(())?;
        let result: String = promise.finish()?;
        println!("[2] Host function result: {result}");
        Ok(())
    })?;

    println!("[2] OK\n");
    Ok(())
}

/// 验证点 3: oxc strip types
fn test_oxc_strip() -> anyhow::Result<()> {
    println!("--- Test 3: oxc strip types ---");

    // 3a: 带 export 的 TS（验证 strip 输出，export 是 module 语法不能直接 eval）
    let ts_module = r#"
        interface Panel { title: string; value: number; }
        export async function getConfig(): Promise<{ title: string; panels: Panel[] }> {
            const x: number = 42;
            return JSON.stringify({ title: "ok", panels: [{ title: "p", value: x }] });
        }
    "#;
    let js_module = strip_typescript(ts_module)?;
    println!("[3a] module JS output:\n{js_module}");

    // 3b: 无 export 的 TS（验证转译后 JS 能被 QuickJS eval）
    let ts_script = r#"
        interface Panel { title: string; value: number; }
        async function getConfig(): Promise<{ title: string; panels: Panel[] }> {
            const x: number = 42;
            return JSON.stringify({ title: "ok", panels: [{ title: "p", value: x }] });
        }
    "#;
    let js_script = strip_typescript(ts_script)?;
    println!("[3b] script JS output:\n{js_script}");

    let rt = Runtime::new()?;
    let ctx = Context::full(&rt)?;
    ctx.with(|ctx| -> anyhow::Result<()> {
        ctx.eval::<(), _>(js_script.as_str())?;
        println!("[3b] JS eval OK");

        // 调用 getConfig 验证完整链路
        let func: Function = ctx.globals().get("getConfig")?;
        let promise: Promise = func.call(())?;
        let result: String = promise.finish()?;
        println!("[3b] getConfig result: {result}");
        Ok(())
    })?;

    println!("[3] OK\n");
    Ok(())
}

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
        anyhow::bail!("parse errors: {:?}", parser_ret.diagnostics);
    }
    let mut program = parser_ret.program;

    // 1) 构建语义模型供 Transformer 使用
    let semantic_ret = SemanticBuilder::new().build(&program);
    if !semantic_ret.diagnostics.is_empty() {
        anyhow::bail!("semantic errors: {:?}", semantic_ret.diagnostics);
    }
    let scoping = semantic_ret.semantic.into_scoping();

    // 2) 默认 TransformOptions 含 TypeScript 类型剥离
    let options = oxc_transformer::TransformOptions::default();
    let transformer_ret =
        Transformer::new(&allocator, std::path::Path::new("config.ts"), &options)
            .build_with_scoping(scoping, &mut program);
    if !transformer_ret.diagnostics.is_empty() {
        anyhow::bail!("transform errors: {:?}", transformer_ret.diagnostics);
    }

    // 3) 代码生成
    let codegen_ret = Codegen::new().build(&program);
    Ok(codegen_ret.code)
}

/// 验证点 4: serde_json::Value 与 JS 值互转（通过 JSON.stringify/parse 桥接）
fn test_serde_bridge() -> anyhow::Result<()> {
    println!("--- Test 4: serde_json bridge ---");
    let rt = Runtime::new()?;
    let ctx = Context::full(&rt)?;

    ctx.with(|ctx| -> anyhow::Result<()> {
        // JS 对象 -> Rust serde_json::Value
        let js_expr = r#"JSON.stringify({ name: "test", count: 42, items: [1, 2, 3] })"#;
        let json_str: String = ctx.eval::<String, _>(js_expr)?;
        let val: serde_json::Value = serde_json::from_str(&json_str)?;
        println!("[4] JS -> serde_json: {val}");

        // Rust serde_json::Value -> JS
        let rust_val = serde_json::json!({ "hello": "world", "n": 100 });
        let js_str = rust_val.to_string();
        let js_val: rquickjs::Value = ctx.eval::<rquickjs::Value, _>(
            format!("JSON.parse({:?})", js_str).as_str(),
        )?;
        ctx.globals().set("myVar", js_val)?;
        let back: String = ctx.eval::<String, _>("JSON.stringify(myVar)")?;
        println!("[4] serde_json -> JS -> string: {back}");
        Ok(())
    })?;

    println!("[4] OK\n");
    Ok(())
}

/// 验证点 5: 完整生产链路 TS export default -> oxc strip -> module eval -> get default -> call
fn test_module_export() -> anyhow::Result<()> {
    println!("--- Test 5: module export default ---");
    let ts = r#"
        interface Panel { title: string; value: number; }
        interface Config { title: string; panels: Panel[]; }

        export default async function getConfig(): Promise<string> {
            const x: number = 42;
            return JSON.stringify({ title: "dashboard", panels: [{ title: "p", value: x }] });
        }
    "#;

    let js = strip_typescript(ts)?;
    println!("[5] stripped JS:\n{js}");

    let rt = Runtime::new()?;
    let ctx = Context::full(&rt)?;
    ctx.with(|ctx| -> anyhow::Result<()> {
        // 用 Module API 处理 export default
        let module = Module::declare(ctx.clone(), "config", js.as_str())?;
        let (evaluated, promise) = module.eval()?;
        // 驱动 module 评估完成
        promise.finish::<()>()?;

        // 取 export default 的值
        let config_func: Function = evaluated.get("default")?;
        println!("[5] got default export: {:?}", config_func.type_of());

        // 调用 async 函数
        let call_promise: Promise = config_func.call(())?;
        let result: String = call_promise.finish()?;
        println!("[5] config result: {result}");
        Ok(())
    })?;

    println!("[5] OK\n");
    Ok(())
}
