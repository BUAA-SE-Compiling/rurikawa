use difference::{Changeset, Difference};

/// Returns if the two `&str`s are **different**, along with a diff String of the two.
///
/// # Examples
/// ```rust
/// use rurikawa_judger::tester::utils::diff;
///
/// let s1 = "Hello,\nworld!\nHi!";
/// let s2 = "Hello,\nthis cruel\nworld!";
/// assert_eq!(
///    dbg!(diff(s1, s2)),
///    (
///        true,
///        "  \
///        Hello,\n\
///        + this cruel\n  \
///        world!\n\
///        - Hi!\n"
///            .into()
///    )
/// );
/// assert_eq!(
///    dbg!(diff(s1, s1)),
///    (
///        false,
///        "  \
///        Hello,\n  \
///        world!\n  \
///        Hi!\n"
///            .into()
///    )
/// );
/// ```
pub fn diff<'a>(got: &'a str, expected: &'a str) -> (bool, String) {
    let changeset = Changeset::new(got, expected, "\n");
    let mut changes = String::new();
    let mut different = false;

    let mut add_diff_ln = |ic: char, s: &str| {
        for ln in s.lines() {
            changes.push_str(&format!("{} {}\n", ic, ln))
        }
    };

    for diff in changeset.diffs {
        match diff {
            Difference::Same(s) => add_diff_ln(' ', &s),
            Difference::Add(s) => {
                add_diff_ln('+', &s);
                different = true;
            }
            Difference::Rem(s) => {
                add_diff_ln('-', &s);
                different = true
            }
        }
    }

    (different, changes)
}

/// Describes a signal code (>=0) in `unix`. Returns [`None`] otherwise.
///
/// # Examples
/// ```rust
/// #[cfg(unix)]
/// {
///     use rurikawa_judger::tester::utils::strsignal;
///
///     let sig = strsignal(1);
///     assert_eq!(dbg!(sig), Some("SIGHUP"));
/// }
/// ```
#[cfg(unix)]
pub fn strsignal(signal: i32) -> Option<&'static str> {
    use nix::sys::signal::Signal;
    use std::convert::TryFrom;
    Signal::try_from(signal).ok().map(|sig| sig.as_str())
}

/// Describes a signal code (>=0) in `unix`. Returns [`None`] otherwise.
///
/// # Examples
/// ```rust
/// #[cfg(not(unix))]
/// {
///     use rurikawa_judger::tester::utils::strsignal;
///
///     let sig = strsignal(1);
///     assert_eq!(dbg!(sig), None);
/// }
/// ```
#[cfg(not(unix))]
pub fn strsignal(signal: i32) -> Option<&'static str> {
    None
}

/// Convert a signal (128-254) to a minus error code, retain the others.
pub fn convert_code(code: i32) -> i32 {
    match code {
        128..=254 => 128 - code,
        _ => code,
    }
}
