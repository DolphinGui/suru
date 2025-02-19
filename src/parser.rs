use std::{collections::HashMap, hash::Hash};

use pest::{
    iterators::{Pair, Pairs},
    Parser,
};
use pest_derive::Parser;

use crate::{functions::eval_function, util::append_string};

#[derive(Parser)]
#[grammar = "bldfile.pest"]
struct BldParser;

#[derive(Default, Debug)]
pub struct Task {
    inputs: Vec<String>,
}

#[derive(Default, Debug)]
pub struct Recipe {
    inputs: Vec<String>,
    steps: Vec<String>,
}


#[derive(Debug)]
pub struct BldFile {
    tasks: HashMap<String, Task>,
    recipes: HashMap<String, Recipe>,
    context: HashMap<String, String>,
}

pub fn parse(input: &str) {
    let mut context = HashMap::new();
    let mut tasks = HashMap::new();
    context.insert("LINKFLAGS".into(), "-MMD -lto -O3".into()); // todo move this later
    let mut v = BldParser::parse(Rule::file, input).unwrap_or_else(|e| panic!("{}", e));
    let a = v.next().unwrap_or_else(|| (panic!()));
    for statement in a.into_inner() {
        match statement.as_rule() {
            Rule::task => {
                let mut inners = statement.into_inner();
                let task = eval_expr_str(
                    inners.next().unwrap_or_else(|| panic!("match task fail")),
                    &context,
                );
                tasks.insert(
                    task,
                    Task {
                        inputs: inners.map(|n| eval_expr_str(n, &context)).collect(),
                    },
                );
            }
            Rule::recipe => {
                // println!("Evaluating recipe: {:?}", statement)
            }
            Rule::vardecl => {
                match_vardecl(&mut statement.into_inner(), &mut context);
                println!("context {:?}", context);
            }
            Rule::EOI => {}
            unknown => panic!("This should never occur {:?}", unknown),
        }
    }
}

fn match_vardecl(var: &mut Pairs<Rule>, context: &mut HashMap<String, String>) {
    let variable = var.next().unwrap_or_else(|| panic!("match vardecl fail"));
    let mut result = String::new();
    for expr in var {
        eval_expr(expr, context, &mut result);
    }
    context.insert(variable.as_str().to_string(), result);
}

fn match_recipe(r: &mut Pairs<Rule>, context: &HashMap<String, String>) -> Recipe {
    let target = r.next().unwrap_or_else(|| panic!("match template fail"));
    let mut inputs = Vec::new();
    let mut steps = Vec::new();
    for stuff in r {
        match stuff.as_rule() {
            Rule::template => inputs.push(stuff.as_str().into()),
            Rule::recipe_step => {
                let mut out = String::new();
                eval_expr(stuff, context, &mut out);
                steps.push(out);
            }
            _ => panic!("This shouldn't happen"),
        }
    }
    Recipe { inputs, steps }
}

fn match_step(step: &mut Pair<Rule>, context: &HashMap<String, String>, out: &mut String){
    match step.as_rule(){
        Rule::expr => {
            // evals_expr(expr, context, result);
        },
        Rule::implicit_var => {
            append_string(out, step.as_str());
        },
        _ => panic!("This shouldn't happen"),
    }
}

fn eval_expr_str(expr: Pair<Rule>, context: &HashMap<String, String>) -> String {
    let mut n = String::new();
    eval_expr(expr, context, &mut n);
    n
}

fn eval_expr(expr: Pair<Rule>, context: &HashMap<String, String>, result: &mut String) {
    assert!(expr.as_rule() == Rule::expr);
    let mut insides = expr.clone().into_inner();
    let inside = insides.next();
    if let Some(first) = inside {
        let others = insides;
        println!("insides: {:?}", others);
        if others.len() != 0 {
            let a = others.map(|a| eval_expr_str(a, context)).collect::<Vec<_>>();
            eval_function(first.as_str(), &a, result)
        } else {
            *result = context.get(first.as_str()).expect("Variable not found").clone();
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
        parse(f);
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
