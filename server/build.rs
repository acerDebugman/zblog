//! Ensure `web/dist` exists so the embedded site compiles from a fresh clone.

use std::path::Path;

const PLACEHOLDER_INDEX: &str = "<!doctype html><title>zblog</title><p>Run `pnpm build` in web/ first.</p>";
const PLACEHOLDER_404: &str = "<!doctype html><title>404</title><p>not found</p>";

fn main() {
    let dist = Path::new("../web/dist");
    if !dist.exists() {
        let result = std::fs::create_dir_all(dist).and_then(|()| {
            std::fs::write(dist.join("index.html"), PLACEHOLDER_INDEX)?;
            std::fs::write(dist.join("404.html"), PLACEHOLDER_404)
        });
        if let Err(e) = result {
            println!("cargo:warning=could not create placeholder web/dist: {e}");
        }
    }
    println!("cargo:rerun-if-changed=../web/dist");
}
