//! The ClubLog developer API key that ships inside the binary.
//!
//! Club Log issue an API key per *application*, not per operator, and the file
//! it guards — cty.xml, the DXCC prefix database every account is classified
//! against — is identical for every server. Before this existed, a fresh dxca
//! install had no country file and classified nothing until an admin went to
//! clublog.org/requestapikey.php, waited for a key to be issued by hand, and
//! pasted it into Settings › Reference data. That was a cold-start wall in
//! front of a value that is the same everywhere.
//!
//! **The key is never in this repository.** `build.rs` reads it at compile
//! time from `DXCA_CLUBLOG_API_KEY` or the git-ignored `.clublog-api-key`, and
//! writes the obfuscated bytes into `OUT_DIR` — under `target/`, not the source
//! tree. Club Log delete keys they find published in a Git repository, and this
//! repo is public. A build with neither source present gets an empty key and
//! behaves exactly as dxca did before: the admin sets one in the web UI.
//!
//! The XOR in `build.rs` is not encryption and is not offered as one. It keeps
//! the key out of `strings` on a shipped binary; anyone determined can still
//! recover it, which is why the answer to a leak is rotation.

use crate::db::Db;
use std::sync::LazyLock;

include!(concat!(env!("OUT_DIR"), "/clublog_key.rs"));

/// The built-in ClubLog developer API key; empty when none was baked in.
pub fn clublog_api_key() -> &'static str {
    static KEY: LazyLock<String> = LazyLock::new(|| {
        OBFUSCATED_CLUBLOG_KEY
            .iter()
            .enumerate()
            .map(|(i, b)| (b ^ CLUBLOG_KEY_PAD[i % CLUBLOG_KEY_PAD.len()]) as char)
            .collect()
    });
    &KEY
}

/// The key cty.xml downloads should actually use.
///
/// An admin-set key wins, so a server whose operator would rather spend their
/// own quota — or one running after Club Log ever revoked the shipped key —
/// needs no new build. Falls back to the built-in one, which is empty in a
/// build that had no key available, leaving the pre-existing behaviour.
pub fn effective_clublog_api_key(db: &Db) -> String {
    let own = db.clublog_api_key();
    if own.trim().is_empty() {
        clublog_api_key().to_string()
    } else {
        own
    }
}

/// Whether this binary carries a key of its own — for the UI to say so without
/// ever being sent the key itself.
pub fn has_builtin_clublog_key() -> bool {
    !clublog_api_key().is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn temp_db() -> (Db, std::path::PathBuf) {
        static N: AtomicU32 = AtomicU32::new(0);
        let path = std::env::temp_dir().join(format!(
            "dxca-builtin-test-{}-{}.db",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = std::fs::remove_file(&path);
        (Db::open(&path).unwrap(), path)
    }

    /// The precedence that decides which key cty.xml downloads with. Getting
    /// this backwards would silently spend the shipped key on a server whose
    /// admin deliberately supplied their own — or, worse, ignore a key set
    /// after ClubLog revoked the built-in one, which is the exact situation the
    /// override exists for.
    #[test]
    fn an_admin_key_wins_and_blank_falls_back_to_the_built_in_one() {
        let (db, path) = temp_db();

        // Nothing set: whatever this build baked in (empty in a plain `cargo
        // test`, a real key in a release build) — either way, the same value.
        assert_eq!(effective_clublog_api_key(&db), clublog_api_key());

        db.set_clublog_api_key("0123456789abcdef0123456789abcdef01234567")
            .unwrap();
        assert_eq!(
            effective_clublog_api_key(&db),
            "0123456789abcdef0123456789abcdef01234567",
            "an admin-set key must override the built-in one"
        );

        // Whitespace is a clear, not a key: a stray space must not become the
        // key sent to cty.php.
        db.set_clublog_api_key("   ").unwrap();
        assert_eq!(
            effective_clublog_api_key(&db),
            clublog_api_key(),
            "a blank-but-present key must fall back, not send spaces"
        );

        db.set_clublog_api_key("").unwrap();
        assert_eq!(effective_clublog_api_key(&db), clublog_api_key());

        let _ = std::fs::remove_file(&path);
    }
}
