//! `{env:VAR}` substitution for configuration values.
//!
//! Lets a config file reference an environment variable instead of embedding a
//! secret, so the same file can be committed and shared safely.

/// Replace every `{env:VAR}` occurrence with the value of `VAR`.
///
/// A `$` immediately before `{env:` suppresses expansion, so a shell-style
/// `${env:VAR}` meant for a script is left alone. A missing variable expands to
/// the empty string rather than failing, so a partially configured file still
/// loads and the missing key surfaces as the usual "no API key" error at
/// request time.
#[must_use]
pub(crate) fn env_expand(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if input[i..].starts_with("{env:") && (i == 0 || bytes[i - 1] != b'$') {
            if let Some(end) = input[i..].find('}') {
                let var = &input[i + 5..i + end];
                out.push_str(&std::env::var(var).unwrap_or_default());
                i += end + 1;
                continue;
            }
        }
        let ch = input[i..].chars().next().expect("char boundary");
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}
