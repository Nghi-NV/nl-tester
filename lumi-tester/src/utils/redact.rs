//! Shared sensitive-data redaction for on-screen text.
//!
//! Used when reading desktop (macOS/Windows) UI content back to a caller/AI -
//! visible text is redacted BEFORE it leaves the parser, not filtered
//! afterwards, so a bug downstream can't leak a value that was never kept.
//!
//! Two signal tiers, checked in order:
//! 1. A real "secure/password" flag from the OS accessibility tree when one
//!    is available (macOS `AXSecureTextField`/`kAXValueIsSecureAttribute`,
//!    Windows UIA `IsPassword`) - always trusted over the heuristic below,
//!    since it is what the app itself declared, not a guess.
//! 2. Shape heuristics: mixed alnum >=6 chars with no spaces (password-like),
//!    a run of >=4 digits (PIN/OTP/card-like), or an email address.
//!
//! A redacted value is always replaced with the fixed-length `MASK_PLACEHOLDER`,
//! never a truncated/partial version of the real value - so no partial leak
//! is possible even if the heuristic under- or over-triggers.
//!
//! This is intentionally a *separate* implementation from
//! `recorder::yaml_generator::YamlGenerator::mask_sensitive_text` (which
//! masks text a user TYPED while recording, with its own tested placeholder
//! format `"********"`/`"****"` baked into generated YAML) - that function's
//! output format is already relied upon by existing tests/behavior and is
//! left untouched. This module is for text READ from the screen instead.

use std::sync::LazyLock;

pub const MASK_PLACEHOLDER: &str = "***MASKED***";

static EMAIL_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}$").unwrap()
});

/// Redacts `text` if it looks sensitive by shape, or unconditionally if
/// `is_secure_field` is true (a real signal from the OS - always wins over
/// the heuristic). Returns `(possibly_redacted_text, was_redacted)`.
pub fn redact_if_sensitive(text: &str, is_secure_field: bool) -> (String, bool) {
    if is_secure_field && !text.is_empty() {
        return (MASK_PLACEHOLDER.to_string(), true);
    }
    if looks_sensitive(text) {
        return (MASK_PLACEHOLDER.to_string(), true);
    }
    (text.to_string(), false)
}

fn looks_sensitive(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return false;
    }

    // Password-like: mixed letters+digits, no spaces, reasonably long.
    if trimmed.len() >= 6 && !trimmed.contains(' ') {
        let has_digits = trimmed.chars().any(|c| c.is_ascii_digit());
        let has_alpha = trimmed.chars().any(|c| c.is_alphabetic());
        if has_digits && has_alpha {
            return true;
        }
    }

    // PIN/OTP/card/phone-like: a run of 4+ digits, allowing the common
    // formatting separators (space, -, ., ,) *between* digits without
    // resetting the run - e.g. "4242 4242 4242 4242" or "512.345" both
    // count as one run. A separator only passes through while a digit run
    // is already open, so it can't chain unrelated numbers across prose
    // ("Section 2. Item 3." stays two separate single-digit runs).
    let mut digit_run = 0;
    for c in trimmed.chars() {
        if c.is_ascii_digit() {
            digit_run += 1;
            if digit_run >= 4 {
                return true;
            }
        } else if digit_run > 0 && matches!(c, ' ' | '-' | '.' | ',') {
            // formatting separator inside an in-progress digit run - keep going
        } else {
            digit_run = 0;
        }
    }

    if EMAIL_RE.is_match(trimmed) {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_password_like_text() {
        let (masked, redacted) = redact_if_sensitive("MyP@ss123", false);
        assert!(redacted);
        assert_eq!(masked, MASK_PLACEHOLDER);
    }

    #[test]
    fn redacts_otp_and_card_like_digit_runs() {
        assert!(redact_if_sensitive("123456", false).1);
        assert!(redact_if_sensitive("Card ending 4242", false).1);
    }

    #[test]
    fn redacts_formatted_numbers_with_separators() {
        // Found via a live test against Calculator's own display - a
        // decimal/grouped number ("512.345") was NOT redacted by the
        // original digit-run check because "." reset the run. Real card/
        // phone numbers are formatted the same way ("4242 4242 4242 4242",
        // "555-123-4567"), so separators must not defeat the check.
        assert!(redact_if_sensitive("512.345", false).1);
        assert!(redact_if_sensitive("4242 4242 4242 4242", false).1);
        assert!(redact_if_sensitive("555-123-4567", false).1);
    }

    #[test]
    fn does_not_chain_unrelated_single_digits_across_prose() {
        assert!(!redact_if_sensitive("Section 2. Item 3. Note 4.", false).1);
    }

    #[test]
    fn redacts_email_addresses() {
        assert!(redact_if_sensitive("user@example.com", false).1);
    }

    #[test]
    fn keeps_ordinary_text_unchanged() {
        let (masked, redacted) = redact_if_sensitive("Hello World", false);
        assert!(!redacted);
        assert_eq!(masked, "Hello World");
    }

    #[test]
    fn keeps_short_labels_and_button_text_unchanged() {
        assert!(!redact_if_sensitive("Save", false).1);
        assert!(!redact_if_sensitive("OK", false).1);
        assert!(!redact_if_sensitive("Settings", false).1);
    }

    #[test]
    fn secure_field_flag_always_redacts_regardless_of_shape() {
        // Even plain-looking text must be masked when the OS itself flags
        // the field as secure - that signal is more trustworthy than any
        // shape heuristic.
        let (masked, redacted) = redact_if_sensitive("hello", true);
        assert!(redacted);
        assert_eq!(masked, MASK_PLACEHOLDER);
    }

    #[test]
    fn empty_text_is_never_redacted() {
        assert!(!redact_if_sensitive("", true).1);
        assert!(!redact_if_sensitive("", false).1);
    }
}
