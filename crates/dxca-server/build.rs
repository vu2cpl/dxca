//! Guarantees `web-ui/dist/` exists before `include_dir!` embeds it, so plain
//! `cargo build` never requires Node/pnpm (Meridian rule). When the real UI
//! has been built (`just web`), its dist is embedded instead; the stub below
//! only ever appears in a binary built on a tree that never ran the web build.
//!
//! Also bakes in the ClubLog developer API key — see `emit_clublog_key`.

use std::fs;
use std::path::Path;

/// XOR pad for the built-in ClubLog key. Not encryption (see below).
///
/// Deliberately high-bit bytes rather than a readable phrase: a pad spelled
/// "…-obfuscation-pad" survives into the binary's rodata verbatim, where
/// `strings` prints it as a signpost standing next to the very bytes it
/// decodes. These produce no printable run, and XOR'd against an ASCII-hex key
/// they leave the key unprintable too.
const PAD: &[u8] = &[
    0xf7, 0xc5, 0xa7, 0xbb, 0xfb, 0xd8, 0xbe, 0xf1, 0xd5, 0xa2, 0x9f, 0xb6, 0xba, 0xe9, 0x84, 0xc5,
    0xe7, 0xc6, 0x91, 0x9d, 0xc3, 0xab, 0xbb, 0xd9, 0xa3, 0xfe, 0x89, 0xd2, 0xa2, 0xf9, 0xad, 0xa1,
];

/// Bake the ClubLog developer API key into the binary, if one is available.
///
/// The key unlocks `cdn.clublog.org/cty.php`, which serves cty.xml — the DXCC
/// prefix database every account is classified against. Club Log issue it per
/// *application*, not per operator, so it is the same value on every dxca
/// server; without it a fresh install has no country file at all and nothing
/// classifies until an admin goes and requests a key by hand.
///
/// **The key is never committed.** Club Log's API Keys article says keys found
/// published on the web or in a Git repository are deleted without notice, and
/// this repository is public. So it is read at build time from the environment
/// (`DXCA_CLUBLOG_API_KEY`) or from `.clublog-api-key` at the repo root, which
/// `.gitignore` covers — and written into `OUT_DIR`, i.e. `target/`, never into
/// the source tree. Nothing to stage, nothing to clear, nothing to forget.
///
/// Release builds (`just dist`, `just win`, `deploy/*.sh`, `install.sh`) pick
/// it up with no extra step, because they all run cargo in the same shell.
/// A build with neither source present gets an empty key and behaves exactly
/// as dxca did before: the admin sets one in Settings › Reference data.
///
/// The XOR is *not* a security measure and is not offered as one — anyone with
/// the binary can recover the bytes. It exists so a shipped `dxca` does not
/// hand the key to `strings`, and so the answer to a leak is rotation rather
/// than pretending it cannot happen.
fn emit_clublog_key(manifest: &str) {
    println!("cargo:rerun-if-env-changed=DXCA_CLUBLOG_API_KEY");
    let key_file = Path::new(manifest).join("../../.clublog-api-key");
    println!("cargo:rerun-if-changed={}", key_file.display());

    let key = std::env::var("DXCA_CLUBLOG_API_KEY")
        .ok()
        .or_else(|| fs::read_to_string(&key_file).ok())
        .unwrap_or_default()
        .trim()
        .to_string();

    if !key.is_empty() && (key.len() != 40 || !key.chars().all(|c| c.is_ascii_hexdigit())) {
        // Wrong shape means a typo'd or truncated key. Failing the build beats
        // shipping a binary whose cty refresh 403s in the field.
        panic!(
            "DXCA_CLUBLOG_API_KEY must be 40 hex characters, got {}",
            key.len()
        );
    }

    let bytes: Vec<String> = key
        .bytes()
        .enumerate()
        .map(|(i, b)| format!("0x{:02x}", b ^ PAD[i % PAD.len()]))
        .collect();
    let pad: Vec<String> = PAD.iter().map(|b| format!("0x{b:02x}")).collect();

    let out = Path::new(&std::env::var("OUT_DIR").unwrap()).join("clublog_key.rs");
    fs::write(
        &out,
        format!(
            "pub(crate) const OBFUSCATED_CLUBLOG_KEY: &[u8] = &[{}];\n\
             pub(crate) const CLUBLOG_KEY_PAD: &[u8] = &[{}];\n",
            bytes.join(", "),
            pad.join(", "),
        ),
    )
    .expect("write clublog_key.rs");
}

fn main() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let dist = Path::new(&manifest).join("../../web-ui/dist");
    let index = dist.join("index.html");
    if !index.exists() {
        fs::create_dir_all(&dist).expect("create web-ui/dist");
        fs::write(
            &index,
            "<!doctype html><meta charset=\"utf-8\"><title>DXCA</title>\
             <body style=\"font-family:system-ui;background:#0d1117;color:#c9d1d9;\
             display:grid;place-items:center;height:100vh;margin:0\">\
             <div><h1>DXCA</h1><p>Web UI not built into this binary — run \
             <code>just web</code> and rebuild.</p></div>",
        )
        .expect("write stub index.html");
    }
    println!("cargo:rerun-if-changed={}", dist.display());

    emit_clublog_key(&manifest);
}
