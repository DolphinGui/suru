pub fn append_string(s: &mut String, end: &str) {
    s.reserve(end.len() + 1);
    if !s.is_empty() {
        s.push(' ');
    }
    s.push_str(end);
}
