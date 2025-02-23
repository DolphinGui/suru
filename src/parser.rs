use std::{collections::HashMap, hash::Hash};

use pest::{
    iterators::{Pair, Pairs},
    Parser,
};
use pest_derive::Parser;

use crate::functions::eval_function;
use crate::util::remove_prefix;

#[derive(Parser)]
#[grammar = "sufile.pest"]
struct TaskParser;

#[derive(Default, Debug, PartialEq, Eq)]
pub struct Task {
    pub inputs: Vec<String>,
}

#[derive(Default, Debug, PartialEq, Eq)]
pub struct Recipe {
    pub templ_in: Vec<String>,
    pub any_in: Vec<String>,
    pub steps: Vec<Vec<String>>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct TaskFile {
    pub tasks: HashMap<String, Task>,
    pub recipes: HashMap<String, Vec<Recipe>>,
}

pub fn parse(
    input: &str,
    context: &mut HashMap<String, Vec<String>>,
    base: &mut TaskFile,
    filename: &str,
) {
    let TaskFile { tasks, recipes } = base;
    let mut input = TaskParser::parse(Rule::file, input)
        .unwrap_or_else(|e| panic!("error parsing {}, {}", filename, e));
    let file = input.next().unwrap_or_else(|| (panic!()));
    for statement in file.into_inner() {
        match statement.as_rule() {
            Rule::task => {
                let mut inners = statement.into_inner();
                let task = fst(&eval_expr(
                    &inners.next().unwrap_or_else(|| panic!("match task fail")),
                    &context,
                ));
                let t = tasks.entry(task).or_default();
                inners
                    .map(|n| eval_expr(&n, &context))
                    .filter(|v| !v.is_empty())
                    .for_each(|mut v| t.inputs.append(&mut v));
            }
            Rule::recipe => {
                let (r, s) = match_recipe(&mut statement.into_inner(), &context);
                recipes.entry(r).or_default().push(s);
            }
            Rule::vardecl => {
                match_vardecl(&mut statement.into_inner(), context);
            }
            Rule::EOI => {}
            unknown => panic!("This should never occur {:?}", unknown),
        }
    }
}

fn match_vardecl(var: &mut Pairs<Rule>, context: &mut HashMap<String, Vec<String>>) {
    let variable = var.next().unwrap_or_else(|| panic!("match vardecl fail"));
    let result = var.flat_map(|expr| match_step(&expr, context)).collect();
    context.insert(variable.as_str().to_string(), result);
}

fn match_recipe(
    recipe: &mut Pairs<Rule>,
    context: &HashMap<String, Vec<String>>,
) -> (String, Recipe) {
    let target = recipe
        .next()
        .unwrap_or_else(|| panic!("match template fail"));
    let mut templ_in = Vec::new();
    let mut any_in = Vec::new();
    let mut steps = Vec::new();
    for stuff in recipe {
        match stuff.as_rule() {
            Rule::template => {
                if stuff.as_str().contains('*') {
                    any_in.push(remove_prefix(stuff.as_str()).to_owned());
                } else if stuff.as_str().contains('%') {
                    templ_in.push(remove_prefix(stuff.as_str()).to_owned());
                } else {
                    panic!();
                }
            }
            Rule::recipe_step => {
                steps.push(
                    stuff
                        .into_inner()
                        .flat_map(|e| match_step(&e, context))
                        .collect(),
                );
            }
            _ => panic!("This shouldn't happen"),
        }
    }
    templ_in.dedup();
    any_in.dedup();
    (
        remove_prefix(target.as_str()).to_owned(),
        Recipe {
            templ_in,
            any_in,
            steps,
        },
    )
}

fn match_step(step: &Pair<Rule>, context: &HashMap<String, Vec<String>>) -> Vec<String> {
    match step.as_rule() {
        Rule::expr => eval_expr(step, context),
        Rule::implicit_var => vec![step.as_str().to_string()],
        _ => panic!("This shouldn't happen"),
    }
}

fn eval_expr(expr: &Pair<Rule>, context: &HashMap<String, Vec<String>>) -> Vec<String> {
    assert!(expr.as_rule() == Rule::expr);
    let mut insides = expr.clone().into_inner();
    let inside = insides.next();
    if let Some(first) = inside {
        let others = insides;
        if others.len() != 0 {
            let a = others
                .flat_map(|a| eval_expr(&a, context))
                .collect::<Vec<_>>();
            eval_function(first.as_str(), &a)
        } else {
            context
                .get(first.as_str())
                .expect(&format!("Variable {} not found", first.as_str()))
                .clone()
        }
    } else {
        vec![expr.as_str().to_string()]
    }
}

fn fst(vec: &Vec<String>) -> String {
    assert!(vec.len() == 1);
    vec[0].clone()
}

#[cfg(test)]
mod test {
    use crate::util::make_svec;

    use super::*;

    #[test]
    fn try_parse() {
        let f = include_str!("test/tasks.su");
        let mut context = HashMap::new();
        context.insert(
            "LINKFLAGS".into(),
            vec!["-MMD", "-lto", "-O3"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
        );

        let mut result = TaskFile::default();
        parse(f, &mut context, &mut result, "test");
        let expected = TaskFile {
            tasks: HashMap::from([(
                "a.exe".into(),
                Task {
                    inputs: make_svec(&["main.o", "lib/lib.o"]),
                },
            )]),
            recipes: HashMap::from([(
                "o".into(),
                vec![
                    Recipe {
                        templ_in: make_svec(&["c"]),
                        any_in: vec![],
                        steps: vec![make_svec(&[
                            "gcc", "-c", "-o", "$@", "$^", "-O3", "-MMD", "-LTO", "-O3",
                        ])],
                    },
                    Recipe {
                        templ_in: make_svec(&["cpp"]),
                        any_in: vec![],
                        steps: vec![make_svec(&[
                            "g++", "-c", "-o", "$@", "$^", "-O3", "-MMD", "-LTO", "-O3",
                        ])],
                    },
                ],
            )]),
        };
        assert_eq!(
            result, expected,
            "Expected {:#?}\ngot {:#?}",
            expected, result
        );
    }

    #[test]
    fn parse_file() {
        let f = include_str!("test/tasks.su");
        let v = TaskParser::parse(Rule::file, f).unwrap_or_else(|e| panic!("{}", e));
        for p in v {
            println!("{:?}", p);
        }
    }

    #[test]
    fn parse_expr() {
        let var = "$(upper $(FLAGS))";
        let mut v = TaskParser::parse(Rule::expr, var).unwrap_or_else(|e| panic!("{}", e));
        let pair = v.next();
        println!("{:?}", pair);
    }

    #[test]
    fn parse_expr_win() {
        let var = "C:\\filename";
        let mut v = TaskParser::parse(Rule::expr, var).unwrap_or_else(|e| panic!("{}", e));
        let pair = v.next();
        println!("{:?}", pair);
    }

    #[test]
    fn parse_task() {
        let task = "a.out: main.o src/other.o";
        let v = TaskParser::parse(Rule::task, task).unwrap_or_else(|e| panic!("{}", e));
        println!("Parsing thing");
        for p in v {
            println!("[{:?}]", p);
        }
    }

    #[test]
    fn parse_recipe() {
        let recipe = "%.h < %.h.in\n\tgenerate.py $@ $^";
        let v = TaskParser::parse(Rule::recipe, recipe).unwrap_or_else(|e| panic!("{}", e));
        for p in v {
            println!("[{:?}]", p);
        }
    }

    #[test]
    fn parse_template() {
        let template = "%.exe";
        let v = TaskParser::parse(Rule::template, template).unwrap_or_else(|e| panic!("{}", e));
        for p in v {
            println!("[{:?}]", p);
        }
    }
}
