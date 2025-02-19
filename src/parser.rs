use std::{collections::HashMap, hash::Hash};

use std::collections::hash_map::Entry::{Occupied, Vacant};

use pest::{
    iterators::{Pair, Pairs},
    Parser,
};
use pest_derive::Parser;

use crate::{functions::eval_function, util::append_string};

#[derive(Parser)]
#[grammar = "bldfile.pest"]
struct BldParser;

#[derive(Default, Debug, PartialEq, Eq)]
pub struct Task {
    pub inputs: Vec<String>,
}

#[derive(Default, Debug, PartialEq, Eq)]
pub struct Recipe {
    pub inputs: Vec<String>,
    pub steps: Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct BldFile {
    pub tasks: HashMap<String, Task>,
    pub recipes: HashMap<String, Recipe>,
}

pub fn parse(input: &str, mut context: HashMap<String, String>) -> BldFile {
    let mut tasks = HashMap::new();
    let mut recipes = HashMap::new();
    let mut v = BldParser::parse(Rule::file, input).unwrap_or_else(|e| panic!("{}", e));
    let a = v.next().unwrap_or_else(|| (panic!()));
    for statement in a.into_inner() {
        match statement.as_rule() {
            Rule::task => {
                let mut inners = statement.into_inner();
                let task = eval_expr_str(
                    &inners.next().unwrap_or_else(|| panic!("match task fail")),
                    &context,
                );
                match tasks.entry(task) {
                    Vacant(vacant) => {
                        vacant.insert(Task {
                            inputs: inners.map(|n| eval_expr_str(&n, &context)).collect(),
                        });
                    }
                    Occupied(mut existing) => {
                        existing
                            .get_mut()
                            .inputs
                            .extend(inners.map(|n| eval_expr_str(&n, &context)));
                    }
                }
            }
            Rule::recipe => {
                let (r, s) = match_recipe(&mut statement.into_inner(), &context);
                recipes.insert(r, s);
            }
            Rule::vardecl => {
                match_vardecl(&mut statement.into_inner(), &mut context);
            }
            Rule::EOI => {}
            unknown => panic!("This should never occur {:?}", unknown),
        }
    }
    BldFile { tasks, recipes }
}

fn match_vardecl(var: &mut Pairs<Rule>, context: &mut HashMap<String, String>) {
    let variable = var.next().unwrap_or_else(|| panic!("match vardecl fail"));
    let mut result = String::new();
    for expr in var {
        eval_expr(&expr, context, &mut result);
    }
    context.insert(variable.as_str().to_string(), result);
}

fn match_recipe(recipe: &mut Pairs<Rule>, context: &HashMap<String, String>) -> (String, Recipe) {
    let target = recipe
        .next()
        .unwrap_or_else(|| panic!("match template fail"));
    let mut inputs = Vec::new();
    let mut steps = Vec::new();
    for stuff in recipe {
        match stuff.as_rule() {
            Rule::template => inputs.push(remove_percent(stuff.as_str())),
            Rule::recipe_step => {
                let mut out = String::new();
                stuff
                    .into_inner()
                    .for_each(|e| match_step(&e, context, &mut out));
                steps.push(out);
            }
            _ => panic!("This shouldn't happen"),
        }
    }
    (remove_percent(target.as_str()), Recipe { inputs, steps })
}

fn remove_percent(file: &str) -> String{
    let mut extension = file.to_string();
    if let Some(ext) = extension.find(".") {
        extension.drain(..ext);
    }
    extension
}

fn match_step(step: &Pair<Rule>, context: &HashMap<String, String>, out: &mut String) {
    match step.as_rule() {
        Rule::expr => {
            eval_expr(step, context, out);
        }
        Rule::implicit_var => {
            append_string(out, step.as_str());
        }
        _ => panic!("This shouldn't happen"),
    }
}

fn eval_expr_str(expr: &Pair<Rule>, context: &HashMap<String, String>) -> String {
    let mut n = String::new();
    eval_expr(expr, context, &mut n);
    n
}

fn eval_expr(expr: &Pair<Rule>, context: &HashMap<String, String>, result: &mut String) {
    assert!(expr.as_rule() == Rule::expr);
    let mut insides = expr.clone().into_inner();
    let inside = insides.next();
    if let Some(first) = inside {
        let others = insides;
        if others.len() != 0 {
            let a = others
                .map(|a| eval_expr_str(&a, context))
                .collect::<Vec<_>>();
            eval_function(first.as_str(), &a, result)
        } else {
            append_string(
                result,
                context
                    .get(first.as_str())
                    .expect(&format!("Variable {} not found", first.as_str())),
            );
        }
    } else {
        append_string(result, expr.as_str());
    }
}

mod test {
    use super::*;

    #[test]
    fn try_parse() {
        let f = include_str!("tasks.bld");
        let mut context = HashMap::new();
        context.insert("LINKFLAGS".into(), "-MMD -lto -O3".into());
        let result = parse(f, context);
        let expected = BldFile {
            tasks: HashMap::from([(
                "a.exe".into(),
                Task {
                    inputs: vec!["main.o".to_string(), "../folder/f.o".to_string()],
                },
            )]),
            recipes: HashMap::from([(
                ".exe".into(),
                Recipe {
                    inputs: vec![".input".to_string()],
                    steps: vec!["gcc -o $@ $^ -O3 -MMD -LTO -O3".to_string()],
                },
            )]),
        };
        assert!(
            result == expected,
            "Expected is not equal parsed,\n got {:?},\n expected {:?}",
            result,
            expected
        );
    }

    #[test]
    fn parse_file() {
        let f = include_str!("tasks.bld");
        let v = BldParser::parse(Rule::file, f).unwrap_or_else(|e| panic!("{}", e));
        for p in v {
            println!("{:?}", p);
        }
    }

    #[test]
    fn parse_expr() {
        let var = "$(upper $(FLAGS))";
        let mut v = BldParser::parse(Rule::expr, var).unwrap_or_else(|e| panic!("{}", e));
        let pair = v.next();
        println!("{:?}", pair);
    }

    #[test]
    fn parse_task() {
        let task = "a.out: main.o src/other.o";
        let v = BldParser::parse(Rule::task, task).unwrap_or_else(|e| panic!("{}", e));
        println!("Parsing thing");
        for p in v {
            println!("[{:?}]", p);
        }
    }

    #[test]
    fn parse_recipe() {
        let recipe = "%.h < %.h.in\n\tgenerate.py $@ $^";
        let v = BldParser::parse(Rule::recipe, recipe).unwrap_or_else(|e| panic!("{}", e));
        for p in v {
            println!("[{:?}]", p);
        }
    }

    #[test]
    fn parse_template() {
        let template = "%.exe";
        let v = BldParser::parse(Rule::template, template).unwrap_or_else(|e| panic!("{}", e));
        for p in v {
            println!("[{:?}]", p);
        }
    }
}
