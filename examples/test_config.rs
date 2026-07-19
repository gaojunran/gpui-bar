use std::path::PathBuf;

fn main() {
    let path: PathBuf = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
            PathBuf::from(home).join(".config/gpui-dashboard/dashboard.config.ts")
        });
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
