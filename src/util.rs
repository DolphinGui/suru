

pub fn append_string(s: &mut String, end: &str) {
    s.reserve(end.len() + 1);
    if !s.is_empty() {
        s.push(' ');
    }
    s.push_str(end);
}

pub fn remove_prefix(file: &str) -> String {
    let mut extension = file.to_string();
    if let Some(ext) = extension.find(".") {
        extension.drain(..ext);
    }
    extension
}

#[cfg(test)]
pub fn make_svec(s: &[&str]) -> Vec<String> {
    s.iter().map(|s| s.to_string()).collect()
}

#[cfg(test)]
use std::collections::HashSet;
use std::ffi::OsStr;
#[cfg(test)]
pub fn make_sset(s: &[&str]) -> HashSet<String> {

    let mut h = HashSet::new();
    h.reserve(s.len());
    for st in s.iter().map(|s| s.to_string()) {
        h.insert(st);
    }
    h
}
