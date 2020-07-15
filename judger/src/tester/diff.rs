use difference::{Changeset, Difference};

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
