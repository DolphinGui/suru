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

use crossbeam::queue::SegQueue;
use indicatif::{MultiProgress, ProgressBar};
use log::{error, info};
use threadpool::ThreadPool;

use crate::hooks::{post_compile, pre_compile, HookResult};
use crate::once_fallible::OnceFallible;
use crate::util::remove_suffix;
use crate::{
    parser::{Recipe, Task, TaskFile},
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

pub fn compile(mut input: TaskFile, builddir: &Path, sourcedir: &Path, mp: MultiProgress) {
    input.tasks = fix_paths(input.tasks, sourcedir, builddir);
    add_implicit(&mut input.tasks, &input.recipes, sourcedir);
    let progress = ProgressBar::new(input.tasks.len() as u64);

    mp.add(progress.clone());

    let roots = get_roots(input.tasks);

    let runner = leak(ThreadPool::new(num_cpus::get_physical()));

    let recipes = leak(input.recipes);

    let builddir = leak(builddir.to_path_buf());
    let sourcedir = leak(sourcedir.to_path_buf());
    let die = Arc::new(AtomicBool::new(false));

    let hook_state = leak(SegQueue::new());

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
                        &hook_state,
                    )
                });
            }
        });
    }

    runner.join();
    post_compile(hook_state, &builddir);
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

fn fix_paths(
    tasks: HashMap<String, Task>,
    sourcedir: &Path,
    builddir: &Path,
) -> HashMap<String, Task> {
    HashMap::from_iter(tasks.into_iter().map(|(f, t)| {
        (
            decannonicalize(f, sourcedir, builddir),
            Task {
                inputs: t
                    .inputs
                    .into_iter()
                    .map(|d| decannonicalize(d, sourcedir, builddir))
                    .collect(),
            },
        )
    }))
}

fn decannonicalize(s: String, sourcedir: &Path, builddir: &Path) -> String {
    let mut source = sourcedir.as_os_str().to_str().unwrap().to_owned();
    if !source.ends_with('/') {
        source.push('/');
    }
    let mut build = builddir.as_os_str().to_str().unwrap().to_owned();
    if !build.ends_with('/') {
        build.push('/');
    }
    if s.starts_with(&build) {
        s.replace(&build, "")
    } else if s.starts_with(&source) {
        s.replace(&source, "")
    } else {
        s
    }
}

fn add_implicit(
    tasks: &mut HashMap<String, Task>,
    recipes: &HashMap<String, Vec<Recipe>>,
    sourcedir: &Path,
) {
    let mut implicit = Vec::new();
    for (_, task) in tasks.iter() {
        for dep in &task.inputs {
            if !tasks.contains_key(dep) && !sourcedir.join(dep).exists() {
                if let Some(r) = recipes.get(remove_prefix(dep)) {
                    implicit.push((
                        dep.clone(),
                        Task {
                            inputs: determine_deps(dep, &r, sourcedir),
                        },
                    ));
                }
            }
        }
    }

    for (k, v) in implicit {
        tasks.insert(k, v);
    }
}

fn determine_deps<'a>(filename: &str, recipes: &[Recipe], sourcedir: &Path) -> Vec<String> {
    let results: Vec<_> = recipes
        .iter()
        .filter(|r| {
            r.templ_in.iter().all(|ext| {
                sourcedir
                    .join(remove_suffix(filename).to_string() + "." + ext)
                    .exists()
            })
        })
        .collect();
    if results.len() > 1 {
        error!("Multiple valid recipes for {}: {:?}", filename, recipes);
        panic!();
    } else if let [s] = results[..] {
        s.templ_in
            .iter()
            .map(|ext| remove_suffix(filename).to_string() + "." + ext)
            .collect()
    } else {
        error!("No valid recipes for {}", filename);
        panic!();
    }
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
    hook_out: &'static SegQueue<HookResult>,
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
        .get(remove_prefix(&target.0))
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
            hook_out,
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
                        hook_out,
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
    hook_out: &'static SegQueue<HookResult>,
) {
    let recipe = recipes
        .iter()
        .find(|r| {
            dependencies
                .iter()
                .any(|d| is_dep_listed(file(d), target, r))
        })
        .unwrap_or_else(|| {
            die.store(true, Relaxed);
            panic!("Unable to find recipe to match {}", target);
        });
    for step in &recipe.steps {
        let mut step = step.iter().map(|s| s.into()).collect();
        let target_file = builddir.join(target);
        let dep_paths: Vec<_> = dependencies
            .into_iter()
            .filter(|d| is_dep_listed(file(d), target, recipe))
            .map(|d| append_dep(d, sourcedir, builddir))
            .collect();
        do_replacements(
            &mut step,
            &target_file,
            &dep_paths,
            builddir.as_os_str(),
            sourcedir.as_os_str(),
        );

        pre_compile(hook_out, &step, &dep_paths, &target_file, sourcedir);

        if needs_compiling(&target_file, &dependencies, sourcedir, builddir).unwrap_or_else(|e| {
            die.store(true, Relaxed);
            panic!(
                "IO error when trying to access metadata for {:?}: {}",
                target_file, e
            );
        }) {
            execute(step, builddir, &mut die, &target_file);
            progress.tick();
        }
    }
}

fn is_dep_listed(dep: &str, target: &str, recipe: &Recipe) -> bool {
    recipe
        .templ_in
        .iter()
        .map(|ext| remove_suffix(target).to_string() + "." + ext)
        .any(|f| f == dep)
        || recipe.any_in.iter().any(|ext| dep.ends_with(ext))
}

fn needs_compiling(
    target: &Path,
    dependencies: &[DependencyFile],
    sourcedir: &Path,
    builddir: &Path,
) -> Result<bool, std::io::Error> {
    if !target.exists() {
        return Ok(true);
    }
    let updatetime = target.metadata()?.modified()?;
    for dep in dependencies
        .iter()
        .map(|d| append_dep(d, sourcedir, builddir))
    {
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

fn append_dep(dep: &DependencyFile, sourcedir: &Path, builddir: &Path) -> PathBuf {
    match dep {
        DependencyFile::Source(s) => sourcedir.join(s),
        DependencyFile::Generated(g) => builddir.join(g),
    }
}

fn execute(
    mut command: Vec<OsString>,
    working_dir: &Path,
    die: &mut Arc<AtomicBool>,
    target: &Path,
) {
    info!("Executing command {:?}", command);
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
    dependencies: &[PathBuf],
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
        s.splice(
            p..p + 1,
            dependencies.iter().map(|d| d.as_os_str().to_owned()),
        );
    }
}
