use std::borrow::Cow;

use once_cell::sync::Lazy;
use regex::Regex;

static replacer: Lazy<Regex> = Lazy::new(|| Regex::new(r"[^A-Za-z0-9-._]").unwrap());

/// Transform a random string as a valid docker tag (only containing alpha/num & dashes).
/// 
/// This function replaces invalid characters into double underlines `__`.
pub(crate) fn transform_string_as_docker_tag(s: &str) -> Cow<str> {
    replacer.replace_all(s, "__")
}
