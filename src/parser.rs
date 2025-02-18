use std::{collections::HashMap, hash::Hash};

use pest::{
    iterators::{Pair, Pairs},
    Parser,
};
use pest_derive::Parser;

use crate::functions::eval_function;

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

#[derive(Default, Debug, PartialEq, Eq, Hash)]
pub struct Variable(String);

#[derive(Debug)]
pub struct BldFile {
    tasks: HashMap<String, Task>,
    recipes: HashMap<String, Recipe>,
    context: HashMap<Variable, String>,
}

pub fn parse(input: &str) {
    let mut context = HashMap::new();
    let mut tasks = HashMap::new();
    context.insert(Variable("LINKFLAGS".into()), "-MMD -lto -O3".into()); // todo move this later
    let mut v = BldParser::parse(Rule::file, input).unwrap_or_else(|e| panic!("{}", e));
    let a = v.next().unwrap_or_else(|| (panic!()));
    for statement in a.into_inner() {
        match statement.as_rule() {
            Rule::task => {
                let mut inners = statement.into_inner();
                let task = eval_expr(
                    inners.next().unwrap_or_else(|| panic!("match task fail")),
                    &context,
                );
                tasks.insert(
                    task,
                    Task {
                        inputs: inners.map(|n| eval_expr(n, &context)).collect(),
                    },
                );
            }
            Rule::recipe => {
                println!("Evaluating recipe: {:?}", statement)
            }
            Rule::vardecl => {
                match_vardecl(&mut statement.into_inner(), &mut context);
            }
            Rule::EOI => {}
            unknown => panic!("This should never occur {:?}", unknown),
        }
    }
}

fn match_vardecl(var: &mut Pairs<Rule>, context: &mut HashMap<Variable, String>) {
    let variable = var.next().unwrap_or_else(|| panic!("match vardecl fail"));
    let mut result = String::new();
    for expr in var {
        result.push_str(&eval_expr(expr, context));
        result.push(' ');
    }
    context.insert(Variable(variable.as_str().to_string()), result);
}

fn match_recipe(r: &mut Pairs<Rule>, context: &HashMap<Variable, String>) -> Recipe {
    let target = r.next().unwrap_or_else(|| panic!("match template fail"));
    let mut inputs = Vec::new();
    let mut steps = Vec::new();
    for stuff in r {
        match stuff.as_rule() {
            Rule::template => inputs.push(stuff.as_str().into()),
            Rule::recipe_step => {
                steps.push(eval_expr(stuff, context))
            },
            _ => panic!("This shouldn't happen"),
        }
    }
    Recipe { inputs, steps }
}

fn eval_expr(expr: Pair<Rule>, context: &HashMap<Variable, String>) -> String {
    assert!(expr.as_rule() == Rule::expr);
    let mut insides = expr.clone().into_inner();
    let inside = insides.next();
    if let Some(first) = inside {
        let others: Vec<_> = insides.map(|e| eval_expr(e, context)).collect();
        if others.len() != 0 {
            eval_function(first.as_str(), &others)
        } else {
            context
                .get(&Variable(first.as_str().to_string()))
                .expect("Undefined variable")
                .clone()
        }
    } else {
        expr.as_str().to_string()
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
        let var = "$(abc)";
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
