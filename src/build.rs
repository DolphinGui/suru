use std::fs;
use std::io::ErrorKind;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::Weak;
use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
    process::Command,
    sync::{atomic::AtomicBool, Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
    time::Duration,
};

use indicatif::{MultiProgress, ProgressBar};
use indicatif_log_bridge::LogWrapper;
use log::{error, info, logger};
use threadpool::ThreadPool;

use crate::once_fallible::OnceFallible;
use crate::{
    parser::{TaskFile, Recipe, Task},
    util::remove_prefix,
};

#[derive(Debug)]
enum DependencyFile {
    Source(String),
    Generated(String),
}

fn file(dep: &DependencyFile) -> &String {
    match dep {
        DependencyFile::Source(s) => s,
        DependencyFile::Generated(s) => s,
    }
}

#[derive(Debug)]
struct Target {
    dependency_files: Vec<DependencyFile>,
    dependents: Vec<Dependent>,
    dependencies: Vec<Weak<(String, DynTarget, OnceFallible)>>,
    is_branch: bool,
}

type DynTarget = RwLock<Target>;
type Dependent = Arc<(String, DynTarget, OnceFallible)>;

pub fn compile(input: TaskFile, builddir: &Path, sourcedir: &Path, mp: MultiProgress) {
    let progress = ProgressBar::new(input.tasks.len() as u64);

    mp.add(progress.clone());

    let roots = get_roots(input.tasks);

    let runner = leak(ThreadPool::new(num_cpus::get_physical()));

    let recipes = leak(input.recipes);

    let builddir = leak(builddir.to_path_buf());
    let sourcedir = leak(sourcedir.to_path_buf());
    let die = Arc::new(AtomicBool::new(false));

    for arc in &roots {
        let arc = arc.1.clone();
        let die = die.clone();
        let progress = progress.clone();
        runner.execute(move || {
            if !arc.2.is_completed() {
                arc.2.call_once_maybe(|| {
                    build_deps(
                        arc.clone(),
                        recipes,
                        runner,
                        &sourcedir,
                        &builddir,
                        die,
                        progress,
                    )
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
                        dependency_files: deps
                            .inputs
                            .into_iter()
                            .map(DependencyFile::Source)
                            .collect(),
                        dependents: Default::default(),
                        dependencies: Default::default(),
                        is_branch: false,
                    }),
                    OnceFallible::new(),
                )),
            )
        }));
    for (_, target_deps) in &unprocessed {
        let mut td = write(&target_deps.1);
        let mut is_branch = false;
        let mut deps: Vec<_> = td
            .dependency_files
            .iter_mut()
            .filter_map(|dep| {
                if let Some(d) = unprocessed.get(file(dep)) {
                    *dep = DependencyFile::Generated(file(dep).clone());
                    write(&d.1).dependents.push(target_deps.clone());
                    is_branch = true;
                    return Some(Arc::downgrade(d));
                }
                None
            })
            .collect();
        td.dependencies.append(&mut deps);
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
    sourcedir: &'static Path,
    builddir: &'static Path,
    die: Arc<AtomicBool>,
    progress: ProgressBar,
) -> bool {
    if die.load(Relaxed) {
        return false;
    }

    for dep in &read_s(&target.1).dependencies {
        let dep = dep.upgrade().expect("Dependency unexpectedly dropped");
        if !dep.2.is_completed() {
            return false; // just give up if we're not ready yet
        }
    }

    let a = recipes
        .get(&remove_prefix(&target.0))
        .or_else(|| recipes.get("%"));
    if let Some(rs) = a {
        run_recipe(
            &target.0,
            &read_s(&target.1).dependency_files,
            rs,
            sourcedir,
            builddir,
            die.clone(),
            &progress,
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
        let progress = progress.clone();
        runner.execute(move || {
            if !arc.2.is_completed() {
                arc.2.call_once_maybe(|| {
                    build_deps(
                        arc.clone(),
                        recipes,
                        runner,
                        sourcedir,
                        builddir,
                        die,
                        progress,
                    )
                });
            }
        });
    }
    true
}

fn run_recipe(
    target: &str,
    dependencies: &[DependencyFile],
    recipes: &[Recipe],
    sourcedir: &Path,
    builddir: &Path,
    mut die: Arc<AtomicBool>,
    progress: &ProgressBar,
) {
    let recipe = recipes
        .iter()
        .find(|r| {
            r.inputs.iter().any(|input| {
                dependencies
                    .iter()
                    .any(|dep| input == &remove_prefix(file(dep)))
            })
        })
        .unwrap_or_else(|| {
            die.store(true, Relaxed);
            panic!("Unable to find recipe to match {}", target);
        });
    for step in &recipe.steps {
        let mut step = step.iter().map(|s| s.into()).collect();
        let filename = builddir.join(target);
        let dependencies: Vec<_> = dependencies
            .iter()
            .map(|d| match d {
                DependencyFile::Source(s) => sourcedir.join(s),
                DependencyFile::Generated(g) => builddir.join(g),
            })
            .collect();
        if needs_compiling(&filename, &dependencies).unwrap_or_else(|e| {
            die.store(true, Relaxed);
            panic!(
                "IO error when trying to access metadata for {:?}: {}",
                filename, e
            );
        }) {
            let dependencies: Vec<_> = dependencies
                .into_iter()
                .filter(|d| recipe.inputs.contains(&remove_prefix(d.to_str().unwrap())))
                .map(|d| d.into_os_string())
                .collect();
            do_replacements(
                &mut step,
                &filename,
                &dependencies,
                builddir.as_os_str(),
                sourcedir.as_os_str(),
            );
            execute(step, builddir, &mut die, &filename);
            progress.tick();
        }
    }
}

fn needs_compiling(target: &Path, dependencies: &[PathBuf]) -> Result<bool, std::io::Error> {
    if !target.exists() {
        return Ok(true);
    }
    let updatetime = target.metadata()?.modified()?;
    for dep in dependencies {
        let a = dep.metadata();
        match a {
            Ok(dep) => {
                if dep.modified()? > updatetime {
                    return Ok(true);
                }
            }
            Err(e) => {
                if e.kind() == ErrorKind::NotFound {
                    return Ok(true);
                }
                return Err(e);
            }
        }
    }
    Ok(false)
}

fn execute(
    mut command: Vec<OsString>,
    working_dir: &Path,
    die: &mut Arc<AtomicBool>,
    target: &Path,
) {
    let cmd = command.remove(0);

    fs::create_dir_all(target.parent().unwrap()).unwrap_or_else(|e| {
        error!(
            "Unable to create parent directories for {:?} due to:\n{}",
            target, e
        );
    });

    let results = Command::new(&cmd)
        .args(&command)
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
                error!("Error running command {:?} {:?}", &cmd, &command);
                panic!(
                    "Build failure code {}:\n{}",
                    out.status,
                    String::from_utf8_lossy(&out.stderr)
                );
            }
            info!(
                "Building {:?}:\n  {}",
                target,
                String::from_utf8_lossy(&out.stderr)
            );
        }
        Err(e) => {
            die.store(true, Relaxed);
            panic!(
                "Error running command when running {:?} {:?}:\n{:?}",
                &cmd, &command, e
            )
        }
    }
}

fn do_replacements(
    s: &mut Vec<OsString>,
    target: &Path,
    dependencies: &[OsString],
    builddir: &OsStr,
    sourcedir: &OsStr,
) {
    s.iter_mut().for_each(|s| {
        if s == "$@" {
            *s = target.as_os_str().into()
        } else if s == "$bd" {
            *s = builddir.into()
        } else if s == "$sd" {
            *s = sourcedir.into()
        }
    });

    while let Some(p) = s.iter().position(|str| str == "$^") {
        s.splice(p..p + 1, dependencies.to_owned());
    }
}
