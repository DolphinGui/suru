pub fn append_string(s: &mut String, end: &str) {
    s.reserve(end.len() + 1);
    if !s.is_empty() {
        s.push(' ');
    }
    s.push_str(end);
}

pub fn remove_prefix(file: &str) -> &str {
    file.split_once('.').unwrap_or(("", file)).1
}

pub fn remove_suffix(file: &str) -> &str {
    file.split_once('.').unwrap_or((file, "")).0
}

#[cfg(test)]
pub fn make_svec(s: &[&str]) -> Vec<String> {
    s.iter().map(|s| s.to_string()).collect()
}
