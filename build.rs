use std::{
    env,
    fs,
    io::Write,
    path::Path,
};

fn main() {
    let bangs_path = "resources/bangs.json";
    println!("cargo:rerun-if-changed={bangs_path}");

    let content = fs::read_to_string(bangs_path).expect("read bangs.json");
    let bangs: Vec<serde_json::Value> = serde_json::from_str(&content).expect("parse bangs.json");

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR");
    let out_path = Path::new(&out_dir).join("builtin_bangs.rs");
    let mut f = fs::File::create(out_path).expect("create builtin_bangs.rs");

    writeln!(f, "static BUILTIN_BANGS: &[(&str, &str, &str)] = &[").expect("write header");
    for bang in &bangs {
        let s = bang["s"].as_str().unwrap_or("");
        let t = bang["t"].as_str().unwrap_or("");
        let u = bang["u"].as_str().unwrap_or("");
        if t.is_empty() || u.is_empty() {
            continue;
        }
        let trigger = format!("!{t}");
        writeln!(f, "    ({s:?}, {trigger:?}, {u:?}),").expect("write bang");
    }
    writeln!(f, "];").expect("write footer");
}
