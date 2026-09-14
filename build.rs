// SPDX-License-Identifier: GPL-3.0-only

use std::process::Command;

fn main() {
    println!("cargo::rerun-if-changed=.git/HEAD");
    println!("cargo::rerun-if-env-changed=COMPASS_VERSION");
    let version = std::env::var("COMPASS_VERSION").unwrap_or_else(|_| git_version());
    println!("cargo::rustc-env=GIT_VERSION={version}");
}

fn git_version() -> String {
    Command::new("git")
        .args(["describe", "--tags", "--always", "--dirty", "--match", "v*"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| {
            String::from_utf8_lossy(&output.stdout)
                .trim()
                .trim_start_matches('v')
                .to_owned()
        })
        .filter(|version| !version.is_empty())
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_owned())
}
