use std::{
    collections::HashMap,
    fmt::Debug,
    path::Path,
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use threadpool::ThreadPool;

use crate::parser::{BldFile, Task};

#[derive(Debug)]
struct Target {
    dependencies: Vec<String>,
    dependents: Vec<DynTarget>,
    is_branch: bool,
}

type DynTarget = Arc<RwLock<Target>>;

pub fn compile(input: BldFile) {
    let roots = get_roots(input.tasks);

    println!("{} roots: {:?}", roots.len(), roots);
    // let runner = ThreadPool::new(threads);
}

fn write(d: &DynTarget) -> RwLockWriteGuard<'_, Target> {
    d.write().expect("This section is single threaded")
}
fn read(d: &DynTarget) -> RwLockReadGuard<'_, Target> {
    d.read().expect("This section is single threaded")
}

fn get_roots(tasks: HashMap<String, Task>) -> Vec<(String, DynTarget)> {
    let unprocessed: HashMap<String, DynTarget> =
        HashMap::from_iter(tasks.into_iter().map(|(file, deps)| {
            (
                file,
                Arc::new(RwLock::new(Target {
                    dependencies: deps.inputs.into_iter().collect(),
                    dependents: Default::default(),
                    is_branch: false,
                })),
            )
        }));

    for (_, target_deps) in &unprocessed {
        let mut td = write(target_deps);
        let mut is_branch = false;
        for dep in td.dependencies.iter() {
            if let Some(d) = unprocessed.get(dep) {
                write(d).dependents.push(target_deps.clone());
                is_branch = true;
            }
        }
        if is_branch {
            td.is_branch = true;
        }
    }

    unprocessed
        .into_iter()
        .filter(|(_, b)| !read(b).is_branch)
        .collect()
}
