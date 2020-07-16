use difference::{Changeset, Difference};
use libc::{c_char, c_int};
use std::ffi::CStr;
use std::str;

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

pub fn strsignal(signal: i32) -> String {
    let c_buf: *const c_char = unsafe { libc::strsignal(signal as c_int) };
    let c_str: &CStr = unsafe { CStr::from_ptr(c_buf) };
    c_str.to_str().unwrap().to_owned()
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

    #[test]
    fn test_strsignal() {
        let e = strsignal(1);
        assert_eq!(dbg!(e), "Hangup: 1");
    }
}
