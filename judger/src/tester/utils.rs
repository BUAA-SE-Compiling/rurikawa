use difference::{Changeset, Difference};
use std::str;

/// Generate a diff String of two Strings.
pub fn diff<'a>(got: &'a str, expected: &'a str) -> (bool, String) {
    let changeset = Changeset::new(got, expected, "\n");
    let mut change_string = String::new();
    let mut different = false;

    let mut add_diff_ln = |ic: char, s: &str| {
        for l in s.lines() {
            change_string.push(ic);
            change_string.push(' ');
            change_string.push_str(l);
            change_string.push('\n');
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

    (different, change_string)
}

#[cfg(not(unix))]
pub fn strsignal(_i: i32) -> Option<&'static str> {
    None
}

#[cfg(unix)]
/// Describe a signal code (>=0).
pub fn strsignal(signal: i32) -> Option<&'static str> {
    use std::convert::TryFrom;
    nix::sys::signal::Signal::try_from(signal)
        .ok()
        .map(|sig| sig.as_str())
}

/// Convert a signal (128-254) to a minus error code, retain the others.
pub fn convert_code(code: i32) -> i32 {
    if (128..=254).contains(&code) {
        128 - code
    } else {
        code
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff() {
        let s1 = "Hello,\nworld!\nHi!";
        let s2 = "Hello,\nthis cruel\nworld!";
        let d = diff(s1, s2);
        assert_eq!(
            dbg!(d),
            (
                true,
                "  \
                Hello,\n\
                + this cruel\n  \
                world!\n\
                - Hi!\n"
                    .into()
            )
        );
    }

    #[test]
    fn test_diff_again() {
        let s1 = "Hello,\nworld!\nHi!";
        let s2 = "Hello,\nworld!\nHi!";
        let d = diff(s1, s2);
        assert_eq!(
            dbg!(d),
            (
                false,
                "  \
                Hello,\n  \
                world!\n  \
                Hi!\n"
                    .into()
            )
        );
    }

    // * `strsignal` is implementation-dependant
    #[cfg(target_os = "macos")]
    #[test]
    fn test_strsignal() {
        let e = strsignal(1).unwrap();
        assert_eq!(dbg!(e), "Hangup: 1");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_strsignal() {
        let e = strsignal(1).unwrap();
        assert_eq!(dbg!(e), "Hangup");
    }
}
