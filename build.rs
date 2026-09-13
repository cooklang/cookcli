use std::path::Path;

/// Front-end assets that are compiled by npm and embedded into the binary by
/// rust-embed. They are gitignored, so a build from a git clone or from
/// GitHub's auto-generated tag archive won't have them — and rust-embed
/// silently embeds nothing, shipping a binary whose web UI renders unstyled
/// (issue #233). Fail the build with instructions instead. Both the crates.io
/// package and the `cook-<version>-source.tar.gz` release asset include the
/// compiled assets, so neither `cargo install cookcli` nor a build from that
/// tarball needs Node.js.
const COMPILED_ASSETS: &[(&str, &str)] = &[
    ("static/css/output.css", "npm run build-css"),
    ("static/js/editor.bundle.js", "npm run build-js"),
];

fn main() {
    let mut missing = Vec::new();
    for (path, command) in COMPILED_ASSETS {
        // Re-runs this check when the file changes or is still missing, and
        // rebuilds the embedded assets after `npm run build-css`/`build-js`.
        println!("cargo:rerun-if-changed={path}");
        if !Path::new(path).exists() {
            missing.push((path, command));
        }
    }

    if missing.is_empty() {
        return;
    }

    eprintln!("error: missing compiled front-end assets:");
    for (path, command) in &missing {
        eprintln!("  - {path}  (generate with: {command})");
    }
    eprintln!(
        "\n\
        These files are generated and not checked into git, so they are absent\n\
        from git clones and from GitHub's auto-generated tag archives.\n\
        Building them requires Node.js:\n\
        \n\
        \tnpm install\n\
        \tnpm run build-css\n\
        \tnpm run build-js\n\
        \n\
        Without them the web UI (`cook server`, `cook build web`) renders\n\
        unstyled pages. To build without Node.js, use a source drop that ships\n\
        them prebuilt: the `cook-<version>-source.tar.gz` asset on the GitHub\n\
        release, or the crates.io package (`cargo install cookcli`)."
    );
    std::process::exit(1);
}
