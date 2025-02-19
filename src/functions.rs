use crate::util::append_string;

pub fn eval_function(name: &str, inputs: &[String], out: &mut String) {
    match name {
        "upper" => upper(inputs, out),
        _ => panic!("Unknown function"),
    }
}

fn upper(inputs: &[String], out: &mut String) {
    let ins = inputs.iter().map(|s| s.to_uppercase());
    for u in ins {
        append_string(out, &u);
    }
}
