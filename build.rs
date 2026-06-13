// Expose the release version to the binary at compile time.
// CI passes APP_VERSION (the git tag, e.g. "0.4.7") as a build arg; locally we
// fall back to the Cargo package version.
fn main() {
    println!("cargo:rerun-if-env-changed=APP_VERSION");
    let version = std::env::var("APP_VERSION")
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());
    println!("cargo:rustc-env=ALERTVIEW_VERSION={version}");
}
