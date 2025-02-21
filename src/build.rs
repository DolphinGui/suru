use std::{
    cell::LazyCell, collections::HashMap, fmt::Debug, path::Path, sync::{Arc, Once, RwLock, RwLockReadGuard, RwLockWriteGuard}, thread::Thread, time::Duration
};

use threadpool::ThreadPool;

use crate::{
    parser::{BldFile, Recipe, Task},
    util::remove_prefix,
};

#[derive(Debug)]
struct Target {
    dependencies: Vec<String>,
    dependents: Vec<Dependent>,
    is_branch: bool,
}

type DynTarget = RwLock<Target>;
type Dependent = Arc<(String, DynTarget, Once)>;

pub fn compile(input: BldFile) {
    let roots = get_roots(input.tasks);

    println!("{} roots: {:?}", roots.len(), roots);
    let runner = Box::leak(Box::new(ThreadPool::new(num_cpus::get_physical()))) as &ThreadPool;

    let recipes = Box::leak(Box::new(input.recipes)) as &HashMap<String, Vec<Recipe>>;

    for arc in &roots {
        let arc = arc.1.clone();
        runner.execute(move || {
            if !arc.2.is_completed() {
                arc.2.call_once(|| {
                    build_deps(arc.clone(), recipes, runner);
                });
            }
        });
    }
    runner.join();
}

fn write(d: &DynTarget) -> RwLockWriteGuard<'_, Target> {
    d.write().expect("This section is single threaded")
}
fn read(d: &DynTarget) -> RwLockReadGuard<'_, Target> {
    d.read().expect("This section is single threaded")
}

fn get_roots(tasks: HashMap<String, Task>) -> Vec<(String, Dependent)> {
    let unprocessed: HashMap<String, Dependent> =
        HashMap::from_iter(tasks.into_iter().map(|(file, deps)| {
            (
                file.clone(),
                Arc::new((
                    file,
                    RwLock::new(Target {
                        dependencies: deps.inputs.into_iter().collect(),
                        dependents: Default::default(),
                        is_branch: false,
                    }),
                    Once::new(),
                )),
            )
        }));

    for (_, target_deps) in &unprocessed {
        let mut td = write(&target_deps.1);
        let mut is_branch = false;
        for dep in td.dependencies.iter() {
            if let Some(d) = unprocessed.get(dep) {
                write(&d.1).dependents.push(target_deps.clone());
                is_branch = true;
            }
        }
        if is_branch {
            td.is_branch = true;
        }
    }

    unprocessed
        .into_iter()
        .filter(|(_, b)| !read(&b.1).is_branch)
        .collect()
}

fn read_s(d: &DynTarget) -> RwLockReadGuard<'_, Target> {
    d.read().expect("This section is read only")
}

fn build_deps(
    target: Dependent,
    recipes: &'static HashMap<String, Vec<Recipe>>,
    runner: &'static ThreadPool,
) {
    let a = recipes.get(&remove_prefix(&target.0));
    if let Some(rs) = a {
        run_recipe(&target.0, &read_s(&target.1).dependencies, rs);
    } else {
        panic!(
            "Could not find a recipe to build {}\n Recipes: {:?}",
            target.0, recipes
        )
    }

    std::thread::sleep(Duration::from_millis(100));

    let target = read_s(&target.1);
    for arc in &target.dependents {
        let arc = arc.clone();
        runner.execute(move || {
            if !arc.2.is_completed() {
                arc.2.call_once(|| {
                    build_deps(arc.clone(), recipes, runner);
                });
            }
        });
    }
}

fn run_recipe(filename: &str, dependencies: &[String], recipes: &[Recipe]) {
    if let Some(recipe) = recipes.iter().find(|r| {
        r.inputs
            .iter()
            .any(|input| dependencies.iter().any(|dep| dep.contains(input)))
    }){
        for step in &recipe.steps{
            let mut step = step.clone();
            do_replacements(&mut step, filename, dependencies);
            println!("building {}: {:?}", filename, step);
        }
    }
}

fn do_replacements(s: &mut Vec<String>, target: &str, dependencies: &[String]){
    s.iter_mut().for_each(|s| if s == "$@" { *s = target.to_string()});
    
    while let Some(p) = s.iter().position(|str| str == "$^"){
        s.splice(p..p+1, dependencies.to_owned());
    }
}
