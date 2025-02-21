use std::sync::atomic::Ordering::Relaxed;
use std::{
    cell::LazyCell,
    collections::HashMap,
    ffi::{OsStr, OsString},
    fmt::Debug,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    str::FromStr,
    sync::{atomic::AtomicBool, Arc, Once, RwLock, RwLockReadGuard, RwLockWriteGuard},
    thread::Thread,
    time::Duration,
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

    let runner = leak(ThreadPool::new(num_cpus::get_physical()));

    let recipes = leak(input.recipes);

    let rootdir = leak(PathBuf::from_str(".").unwrap());
    let rootdir_path = rootdir.as_path();
    let die = Arc::new(AtomicBool::new(false));

    for arc in &roots {
        let arc = arc.1.clone();
        let die = die.clone();
        runner.execute(move || {
            if !arc.2.is_completed() {
                arc.2.call_once(|| {
                    build_deps(arc.clone(), recipes, runner, rootdir_path, die);
                });
            }
        });
    }
    runner.join();
}

fn leak<T>(t: T) -> &'static T {
    Box::leak(Box::new(t))
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
    rootdir: &'static Path,
    die: Arc<AtomicBool>,
) {
    if die.load(Relaxed) {
        return;
    }

    let a = recipes
        .get(&remove_prefix(&target.0))
        .or_else(|| recipes.get("%"));
    if let Some(rs) = a {
        run_recipe(
            &target.0,
            &read_s(&target.1).dependencies,
            rs,
            rootdir,
            die.clone(),
        );
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
        let die = die.clone();
        runner.execute(move || {
            if !arc.2.is_completed() {
                arc.2.call_once(|| {
                    build_deps(arc.clone(), recipes, runner, rootdir, die);
                });
            }
        });
    }
}

fn run_recipe(
    filename: &str,
    dependencies: &[String],
    recipes: &[Recipe],
    rootdir: &Path,
    mut die: Arc<AtomicBool>,
) {
    let recipe = recipes
        .iter()
        .find(|r| {
            r.inputs
                .iter()
                .any(|input| dependencies.iter().any(|dep| input == &remove_prefix(dep)))
        })
        .unwrap_or_else(|| {
            die.store(true, Relaxed);
            panic!("Unable to find recipe to match {}", filename);
        });
    for step in &recipe.steps {
        let mut step = step.iter().map(|s| s.into()).collect();
        let filename = rootdir.join(filename);
        let dependencies: Vec<_> = dependencies
            .iter()
            .filter(|d| recipe.inputs.contains(&remove_prefix(d)))
            .map(|d| rootdir.join(d).into_os_string())
            .collect();
        do_replacements(&mut step, &filename, &dependencies, rootdir.as_os_str());
        println!("Executing {:?}", step);
        execute(step, rootdir, &mut die, filename.as_os_str());
    }
}

fn execute(
    mut command: Vec<OsString>,
    working_dir: &Path,
    die: &mut Arc<AtomicBool>,
    filename: &OsStr,
) {
    let cmd = command.remove(0);
    let results = Command::new(cmd)
        .args(command)
        .current_dir(
            working_dir
                .canonicalize()
                .expect("Unable to cannonicalize rootdir"),
        )
        .output();
    match results {
        Ok(out) => {
            if !out.status.success() {
                die.store(true, Relaxed);
                panic!(
                    "Build failure code {}:\n{}",
                    out.status,
                    String::from_utf8_lossy(&out.stderr)
                );
            }
            println!("{:?}: {}", filename, String::from_utf8_lossy(&out.stderr));
        }
        Err(e) => {
            die.store(true, Relaxed);
            panic!(
                "Error running command when building {:?}:\n{:?}",
                filename, e
            )
        }
    }
}

fn do_replacements(s: &mut Vec<OsString>, target: &Path, dependencies: &[OsString], root: &OsStr) {
    s.iter_mut().for_each(|s| {
        if s == "$@" {
            *s = target.as_os_str().into()
        } else if s == "$br" {
            *s = root.into()
        }
    });

    while let Some(p) = s.iter().position(|str| str == "$^") {
        s.splice(p..p + 1, dependencies.to_owned());
    }
}
