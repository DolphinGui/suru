use std::{
    collections::HashMap,
    fmt::Debug,
    path::Path,
    sync::{Arc, RwLock},
};

use threadpool::ThreadPool;

use crate::parser::{BldFile, Task};

enum Dependency {
    Compiled((String, DynTarget)),
    Source(String),
}

impl Debug for Dependency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Compiled((filename, t)) => {
                write!(f, "cmp {}", filename)
            }
            Self::Source(filename) => write!(f, "src {}", filename),
        }
    }
}

#[derive(Debug)]
struct Target {
    dependencies: Vec<Dependency>,
    is_dep: bool,
}

type DynTarget = Arc<RwLock<Target>>;

pub fn compile(input: BldFile) {
    let roots = get_roots(input.tasks);

    println!("{} roots: {:?}", roots.len(), roots);
    // let runner = ThreadPool::new(threads);
}

fn get_roots(tasks: HashMap<String, Task>) -> Vec<(String, DynTarget)> {
    let unprocessed: HashMap<String, DynTarget> =
        HashMap::from_iter(tasks.into_iter().map(|(file, deps)| {
            (
                file,
                Arc::new(RwLock::new(Target {
                    dependencies: deps.inputs.into_iter().map(Dependency::Source).collect(),
                    is_dep: false,
                })),
            )
        }));

    for (target_file, target_deps) in &unprocessed {
        for (_, deps) in &unprocessed {
            let mut n = deps.write().expect("This section is single-threaded");
            update_deps(&target_file, &target_deps, &mut n)
        }
        let mut n = target_deps
            .write()
            .expect("This section is single-threaded");

        for target_dep in &mut n.dependencies {
            if let Dependency::Source(file) = target_dep {
                if let Some(t) = unprocessed.get(file) {
                    t.write().expect("This is single threaded").is_dep = true;
                    *target_dep = Dependency::Compiled((file.to_string(), t.clone()));
                }
            }
        }
    }

    unprocessed
        .into_iter()
        .filter(|(_, b)| !b.read().unwrap().is_dep)
        .collect()
}

fn update_deps(depname: &str, deptarget: &DynTarget, target: &mut Target) {
    for dep in &mut target.dependencies {
        if let Dependency::Source(file) = dep {
            if depname == file {
                deptarget.write().expect("singled threaded only").is_dep = true;
                *dep = Dependency::Compiled((depname.to_string(), deptarget.clone()));
            }
        }
    }
}
