fn main() {
    let git_sha = std::env::var("OKF_LINT_GIT_SHA")
        .ok()
        .filter(|sha| !sha.is_empty())
        .or_else(git_sha_from_git_rev_parse)
        .or_else(git_sha_from_cargo_vcs_info)
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=OKF_LINT_GIT_SHA={git_sha}");
}

fn git_sha_from_git_rev_parse() -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8(output.stdout).ok()?.trim().to_string())
}

// `cargo package`/`cargo publish` writes this file into the tarball with the sha of the
// commit that was packaged. A `cargo install` build runs from that extracted tarball,
// which has no `.git` directory, so `git_sha_from_git_rev_parse` always misses there —
// this is the only source of the commit in that case.
fn git_sha_from_cargo_vcs_info() -> Option<String> {
    let contents = std::fs::read_to_string(".cargo_vcs_info.json").ok()?;
    let after_key = contents.split("\"sha1\"").nth(1)?;
    let after_colon = after_key.split_once(':')?.1;
    let quoted = after_colon.split_once('"')?.1;
    let sha = quoted.split_once('"')?.0;
    Some(sha.to_string())
}
