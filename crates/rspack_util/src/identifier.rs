use std::{borrow::Cow, path::Path};

use concat_string::concat_string;
use once_cell::sync::Lazy;
use regex::Regex;
use sugar_path::{AsPath, SugarPath};

static SEGMENTS_SPLIT_REGEXP: Lazy<Regex> = Lazy::new(|| Regex::new(r"([|!])").expect("TODO:"));
static WINDOWS_ABS_PATH_REGEXP: Lazy<Regex> =
  Lazy::new(|| Regex::new(r"^[a-zA-Z]:[/\\]").expect("TODO:"));
static WINDOWS_PATH_SEPARATOR_REGEXP: Lazy<Regex> =
  Lazy::new(|| Regex::new(r"[/\\]").expect("TODO:"));
pub fn make_paths_relative(context: &str, identifier: &str) -> String {
  SEGMENTS_SPLIT_REGEXP
    .split(identifier)
    .map(|s| absolute_to_request(context, s))
    .collect::<Vec<_>>()
    .join("")
}

/// # Example
///  ```ignore
/// assert_eq!(
///   split_at_query_mark("/hello?world=1"),
///   ("/hello", Some("?world=1"))
/// )
/// ```
fn split_at_query_mark(path: &str) -> (&str, Option<&str>) {
  let query_mark_pos = path.find('?');
  query_mark_pos
    .map(|pos| (&path[..pos], Some(&path[pos..])))
    .unwrap_or((path, None))
}

// Port from https://github.com/webpack/webpack/blob/4b4ca3bb53f36a5b8fc6bc1bd976ed7af161bd80/lib/util/identifier.js#L30
pub fn absolute_to_request<'b>(context: &str, maybe_absolute_path: &'b str) -> Cow<'b, str> {
  if maybe_absolute_path.starts_with('/')
    && maybe_absolute_path.len() > 1
    && maybe_absolute_path.ends_with('/')
  {
    // this 'path' is actually a regexp generated by dynamic requires.
    // Don't treat it as an absolute path.
    return Cow::Borrowed(maybe_absolute_path);
  }

  let (maybe_absolute_resource, query_part) = split_at_query_mark(maybe_absolute_path);

  let relative_resource = if maybe_absolute_path.starts_with('/') {
    let tmp = Path::new(maybe_absolute_resource).relative(context);
    let tmp_path = tmp.to_string_lossy();
    relative_path_to_request(&tmp_path).into_owned()
  } else if WINDOWS_ABS_PATH_REGEXP.is_match(maybe_absolute_path) {
    let mut resource = maybe_absolute_resource
      .as_path()
      .relative(context)
      .to_string_lossy()
      .into_owned();

    // In windows, A path that relative to a another path could still be absolute.
    // ("d:/aaaa/cccc").relative("c:/aaaaa/") would get "d:/aaaa/cccc".
    if !WINDOWS_ABS_PATH_REGEXP.is_match(&resource) {
      resource =
        relative_path_to_request(&WINDOWS_PATH_SEPARATOR_REGEXP.replace_all(&resource, "/"))
          .into_owned();
    }
    resource
  } else {
    // not an absolute path
    return Cow::Borrowed(maybe_absolute_path);
  };

  return if let Some(query_part) = query_part {
    Cow::Owned(concat_string!(relative_resource, query_part))
  } else {
    Cow::Owned(relative_resource)
  };
}

/// # Context
/// First introduced at https://github.com/webpack/webpack/commit/5563ee9e583602eb38ab21219a327d346cd16218#r120784061
/// Introduced at https://github.com/webpack/webpack/commit/c76be4d7383f35b3260dafefbcd24cac245d9e42
/// Fix https://github.com/webpack/webpack/issues/14014
pub fn relative_path_to_request(rel: &str) -> Cow<str> {
  if rel.is_empty() {
    Cow::Borrowed("./.")
  } else if rel == ".." {
    Cow::Borrowed("../.")
  } else if rel.starts_with("../") {
    Cow::Borrowed(rel)
  } else {
    Cow::Owned(concat_string!("./", rel))
  }
}