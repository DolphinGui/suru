use std::{env, path::PathBuf, str::FromStr};

pub fn eval_function(name: &str, inputs: &[String]) -> Vec<String> {
    match name {
        "upper" => inputs.iter().map(|s| s.to_uppercase()).collect(),
        "exe" => exe(inputs),
        "env" => env(inputs),
        "or" => or(inputs),
        "path" => path(inputs),
        "just" => inputs.to_owned(),
        "first" => first(inputs),
        "last" => last(inputs),
        _ => panic!("Unknown function"),
    }
}

fn exe(inputs: &[String]) -> Vec<String> {
    if inputs.len() != 1 {
        "Expected exactly one argument to the function exe";
    }
    let mut a = inputs[0].clone();
    if cfg!(windows) {
        a.push_str(".exe");
    }
    vec![a]
}

fn env(inputs: &[String]) -> Vec<String> {
    if inputs.len() != 1 {
        "Expected exactly one argument to the function env";
    }
    let var = std::env::var(&inputs[0]);
    match var {
        Ok(val) => vec![val],
        Err(err) => match err {
            std::env::VarError::NotPresent => Vec::new(),
            std::env::VarError::NotUnicode(_) => {
                panic!("Environment variable contained non-unicode data, contact developer")
            }
        },
    }
}

fn path(inputs: &[String]) -> Vec<String> {
    if inputs.len() != 1 {
        "Expected exactly one argument to the function exists";
    }
    let mut input = inputs[0].clone();
    if cfg!(windows) && !input.ends_with(".exe") {
        input.push_str(".exe");
    }
    let path = PathBuf::from_str(&input).expect(&format!(
        "Unable to parse executable {} when evaluating path",
        input
    ));

    env::var_os("PATH")
        .and_then(|paths| {
            env::split_paths(&paths)
                .filter_map(|dir| {
                    if dir.join(&path).is_file() {
                        Some(vec![input.clone()])
                    } else {
                        None
                    }
                })
                .next()
        })
        .unwrap_or_default()
}

fn or(inputs: &[String]) -> Vec<String> {
    inputs.get(0).map(|s| vec![s.clone()]).unwrap_or_default()
}

fn first(inputs: &[String]) -> Vec<String> {
    if inputs.len() < 1 {
        panic!("Expected at least one argument to function first");
    }
    let num: usize = inputs[0]
        .parse()
        .expect(&format!("Parsing error when parsing {}", &inputs[0]));
    if inputs.len() < num + 1 {
        panic!("Expected at least {} arguments for function first", num + 1);
    }

    inputs[1..num+1].to_owned()
}

fn last(inputs: &[String]) -> Vec<String> {
    if inputs.len() == 0 {
        panic!("Expected at least one argument to function last");
    }
    let num: usize = inputs[0]
        .parse()
        .expect(&format!("Parsing error when parsing {}", &inputs[0]));
    if inputs.len() < num + 1 {
        panic!("Expected at least {} arguments for function last", num + 1);
    }

    inputs[(inputs.len() - num)..inputs.len()].to_owned()
}
#[cfg(test)]
mod test {
    use crate::util::make_svec;

    use super::*;

    #[test]
    fn test_first_last() {
        let inputs = make_svec(&["3", "a", "b", "c", "d"]);
        let results = last(&inputs);
        assert_eq!(results, make_svec(&["b", "c", "d"]));

        let results = first(&inputs);
        assert_eq!(results, make_svec(&["a", "b", "c"]));
    }
}
