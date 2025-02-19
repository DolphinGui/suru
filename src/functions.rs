use crate::util::append_string;

pub fn eval_function(name: &str, inputs: &[String])-> Vec<String> {
    match name {
        "upper" => inputs.iter().map(|s| s.to_uppercase()).collect(),
        _ => panic!("Unknown function"),
    }
}

