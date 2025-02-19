use std::{
    collections::HashMap,
    ffi::OsStr,
    path::{Path, PathBuf},
    time::SystemTime,
};

use threadpool::ThreadPool;

use crate::{
    parser::{BldFile, Recipe, Task},
    util::remove_prefix,
};

struct Context<'a> {
    recipes: &'a HashMap<String, Recipe>,
    tasks: &'a HashMap<String, Task>,
    rootdir: &'a Path,
    runner: &'a ThreadPool,
}

struct Target{
    command: (String, Vec<String>),
    dependencies: Vec<Box<Target>>
}

pub fn compile(input: BldFile, rootdir: &Path, threads: usize) {
    let runner = ThreadPool::new(threads);

}

fn parse_task(){}

fn eval_expr(s: &str) {}

fn filter<'a, T>(s: &str, filters: T) -> bool
where
    T: Iterator<Item = &'a String>,
{
    for filter in filters {
        if filter == "%" {
            return true;
        }
        match s.find(filter) {
            Some(_) => return true,
            None => continue,
        }
    }
    false
}


fn get_modified(file: &Path) -> SystemTime {
    std::fs::metadata(file)
        .expect(&format!("Unable to get metadata for file {:?}", file))
        .modified()
        .expect("Unable to access last modification time.")
}
