pub fn eval_function(name: &str, inputs: &[String]) -> String {
    match name {
        "upper" => upper(inputs),
        _ => panic!("Unknown function"),
    }
}

fn upper(inputs: &[String]) -> String {
    let ins = inputs.iter().map(|s| s.to_uppercase());
    let mut result = String::new();
    for u in ins {
        result.push_str(&u);
        result.push(' ');
    }
    result
}
