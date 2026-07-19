use std::path::PathBuf;

fn main() {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    let path = PathBuf::from(home).join(".config/gpui-dashboard/dashboard.config.ts");
    println!("config path: {}", path.display());

    match gpui_dashboard::js_runtime::run_config(&path) {
        Ok(value) => {
            println!("run_config OK");
            println!("{}", serde_json::to_string_pretty(&value).unwrap());
        }
        Err(e) => {
            eprintln!("run_config FAILED: {e:#}");
            std::process::exit(1);
        }
    }
}
