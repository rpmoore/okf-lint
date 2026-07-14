fn main() {
    println!("cargo:rerun-if-env-changed=OKF_LINT_GIT_SHA");
    println!("cargo:rerun-if-changed=.cargo_vcs_info.json");
    watch_git_head();

    let git_sha = std::env::var("OKF_LINT_GIT_SHA")
        .ok()
        .and_then(|sha| valid_git_sha(sha.trim()))
        .or_else(git_sha_from_git_rev_parse)
        .or_else(git_sha_from_cargo_vcs_info)
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=OKF_LINT_GIT_SHA={git_sha}");
}

// `OKF_LINT_GIT_SHA` can come from outside this build (the `Dockerfile`'s `ARG`, or a
// caller's shell environment) — unlike the git-derived sources below, its contents
// aren't guaranteed to be a real commit hash. A stray newline or arbitrary string would
// flow straight into `run_version`'s single-line `commit: {sha}` output, so only accept
// values that look like a git object id (40 hex chars for SHA-1, 64 for SHA-256) and
// fall back to the git-derived sources otherwise.
fn valid_git_sha(sha: &str) -> Option<String> {
    let looks_like_sha = matches!(sha.len(), 40 | 64) && sha.chars().all(|c| c.is_ascii_hexdigit());
    looks_like_sha.then(|| sha.to_string())
}

// `.git/HEAD` only changes on a branch switch (or detached checkout) — the loose ref
// file it points to (e.g. `.git/refs/heads/main`) is what actually moves on every local
// commit or pull, and `.git/packed-refs` is where that same ref can live instead after
// `git gc`/a fresh clone. Watching only `.git/HEAD` (as an earlier version of this build
// script did) leaves the embedded commit stale across ordinary commits on the current
// branch, since Cargo wouldn't know to rerun this script.
fn watch_git_head() {
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/packed-refs");

    let Ok(head) = std::fs::read_to_string(".git/HEAD") else {
        return;
    };
    if let Some(ref_path) = head.trim().strip_prefix("ref: ")
        && is_normal_ref_path(ref_path)
    {
        println!("cargo:rerun-if-changed=.git/{ref_path}");
    }
}

// `.git/HEAD`'s `ref: ` line is meant to hold a path like `refs/heads/main`, but nothing
// stops it from containing something else (a malformed or hand-edited file). Emitting
// `cargo:rerun-if-changed=.git/{ref_path}` unchecked would let a `ref_path` like
// `../../etc/passwd` make Cargo watch a path outside `.git/` entirely — restrict it to
// the normal `refs/...` namespace with no `..` traversal before trusting it.
fn is_normal_ref_path(ref_path: &str) -> bool {
    ref_path.starts_with("refs/") && ref_path.split('/').all(|part| part != "..")
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
