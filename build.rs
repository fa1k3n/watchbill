//! Fixes the folder that holds the files of the revision when `wb` is built.
//!
//! The package gives it in `WB_SHARE` (`/usr/share/watchbill`). Without it, a build in a checkout
//! takes `share/watchbill` of the checkout. The value is part of the program: when `wb` runs, no
//! option and no environment variable changes it.

fn main() {
    println!("cargo:rerun-if-env-changed=WB_SHARE");
    let folder = match std::env::var("WB_SHARE") {
        Ok(folder) if !folder.is_empty() => folder,
        _ => format!("{}/share/watchbill", std::env::var("CARGO_MANIFEST_DIR").expect("cargo gives the folder of the package")),
    };
    println!("cargo:rustc-env=WB_SHARE_FOLDER={}", folder);
}
