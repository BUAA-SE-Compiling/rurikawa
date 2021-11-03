use difference::{Changeset, Difference};
use std::borrow::Cow;

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
                if !s.is_empty() {
                    add_diff_ln('+', &s);
                    different = true;
                }
            }
            Difference::Rem(s) => {
                if !s.is_empty() {
                    add_diff_ln('-', &s);
                    different = true;
                }
            }
        }
    }

    (different, changes)
}

/// Return the description of a signal code (>=0) in Linux. If such description
/// was not found, return a string like `Signal XXX` instead.
///
/// This function uses the common part of the Linux signal table among various
/// architectures.
///
/// See: https://www.man7.org/linux/man-pages/man7/signal.7.html
pub fn strsignal(signal: i32) -> Cow<'static, str> {
    match signal {
        0 => "No error".into(),
        1 => "SIGHUP (Hangup)".into(),
        2 => "SIGINT (Interrupt)".into(),
        3 => "SIGQUIT (Quit)".into(),
        4 => "SIGILL (Illegal Instruction)".into(),
        5 => "SIGTRAP (Trap/Breakpoint Trap)".into(),
        6 => "SIGABRT (Abort)".into(),
        8 => "SIGFPE (Floating-point Exception)".into(),
        9 => "SIGKILL (Kill)".into(),
        11 => "SIGSEGV (Segmentation Fault)".into(),
        14 => "SIGALRM (Alarm)".into(),
        15 => "SIGTERM (Termination)".into(),
        _ => format!("Signal {}", signal).into(),
    }
}

/// Convert a signal (128-254) to a minus error code, retain the others.
pub fn convert_code(code: i32) -> i32 {
    match code {
        128..=254 => 128 - code,
        _ => code,
    }
}
