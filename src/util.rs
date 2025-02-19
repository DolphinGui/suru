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
