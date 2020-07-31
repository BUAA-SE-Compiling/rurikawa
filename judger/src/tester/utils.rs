use difference::{Changeset, Difference};
use libc::{c_char, c_int};
use std::ffi::CStr;
use std::str;

/// Generate a diff String of two Strings.
pub fn diff<'a>(got: &'a str, expected: &'a str) -> String {
    let Changeset { diffs, .. } = Changeset::new(got, expected, "\n");

    fn make_diff_line(ln_diff: &Difference) -> String {
        match ln_diff {
            Difference::Same(ln) => "  ".to_owned() + ln,
            Difference::Rem(ln) => "- ".to_owned() + ln,
            Difference::Add(ln) => "+ ".to_owned() + ln,
        }
    }

    diffs
        .iter()
        .map(make_diff_line)
        .collect::<Vec<String>>()
        .join("\n")
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
/// Describe a signal code (>=0).
pub fn strsignal(signal: i32) -> String {
    let c_buf: *const c_char = unsafe { libc::strsignal(signal as c_int) };
    let c_str: &CStr = unsafe { CStr::from_ptr(c_buf) };
    c_str.to_str().unwrap().to_owned()
}

/// Convert a signal (128-254) to a minus error code, retain the others.
pub fn convert_code(code: i32) -> i32 {
    if 128 <= code && code <= 254 {
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
            "  \
              Hello,\n\
            + this cruel\n  \
              world!\n\
            - Hi!"
        );
    }

    // * `strsignal` is implementation-dependant
    #[cfg(target_os = "macos")]
    #[test]
    fn test_strsignal() {
        let e = strsignal(1);
        assert_eq!(dbg!(e), "Hangup: 1");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_strsignal() {
        let e = strsignal(1);
        assert_eq!(dbg!(e), "Hangup");
    }
}
