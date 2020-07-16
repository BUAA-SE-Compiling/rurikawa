use difference::{Changeset, Difference};
use libc::{c_char, c_int};
use std::ffi::CStr;
use std::future::Future;
use std::str;
use std::time::Duration;
use tokio::runtime;
use tokio::time;

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

pub fn with_timeout<F, O>(timeout: Duration, future: F) -> Result<O, tokio::time::Elapsed>
where
    F: Future<Output = O>,
{
    let res = async { time::timeout(timeout, future).await };
    let mut rt = runtime::Runtime::new().unwrap();
    rt.block_on(res)
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

    #[test]
    fn test_with_timeout() {
        let timeout = |t: u64| {
            with_timeout(Duration::from_millis(t), async {
                tokio::time::delay_for(Duration::from_millis(100)).await;
                println!("100 ms have elapsed");
                "Hi".to_owned()
            })
        };
        let res1 = timeout(50);
        let res2 = timeout(150);

        assert!(res1.is_err());
        assert_eq!(res2, Ok("Hi".to_owned()));
    }
}
