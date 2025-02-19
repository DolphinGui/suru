use std::{
    borrow::Borrow,
    cell::{Ref, RefCell},
    collections::{
        hash_map::Entry::{Occupied, Vacant},
        HashMap, HashSet,
    },
    ffi::OsStr,
    hash::Hash,
    path::{Path, PathBuf},
    rc::Rc,
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

#[derive(Debug, PartialEq, Eq)]
enum Dependency {
    Compiled(DynTarget),
    Source(String),
}

#[derive(Debug, PartialEq, Eq)]
struct Target {
    dependencies: Vec<Dependency>,
    is_dep: bool,
}

type DynTarget = Rc<RefCell<Target>>;

pub fn compile(input: BldFile, rootdir: &Path, threads: usize) {
    let runner = ThreadPool::new(threads);

    let unprocessed: HashMap<String, DynTarget> =
        HashMap::from_iter(input.tasks.into_iter().map(|(file, deps)| {
            (
                file,
                Rc::new(RefCell::new(Target {
                    dependencies: deps.inputs.into_iter().map(Dependency::Source).collect(),
                    is_dep: false,
                })),
            )
        }));

    let mut roots: HashMap<String, DynTarget> = Default::default();
    for (target_file, target_deps) in &unprocessed {
        for (_, deps) in &roots {
            if update_deps(&target_file, &target_deps, &mut deps.borrow_mut()) {
                target_deps.borrow_mut().is_dep = true;
            }
        }
        let target_deps: &mut Target = &mut target_deps.borrow_mut();
        for target_dep in &mut target_deps.dependencies {
            if let Dependency::Source(s) = target_dep{
                
            }
        }
    }
}

fn update_deps(s: &str, t: &DynTarget, root: &mut Target) -> bool {
    let mut updated = false;
    for dep in root.dependencies.iter_mut() {
        if let Dependency::Source(file) = dep {
            if s == file {
                *dep = Dependency::Compiled(t.clone());
                updated = true;
            }
        }
    }
    updated
}

// fn

fn parse_task() {}

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
