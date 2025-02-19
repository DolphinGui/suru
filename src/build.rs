use std::{collections::HashMap, ffi::OsStr, fs::File, path::Path, time::SystemTime};

use crate::parser::{BldFile, Recipe, Task};

pub fn compile(input: BldFile, rootdir: &Path) {
    for (filename, task) in input.tasks {
        run_task(Path::new(&filename), task, &input.recipes, rootdir);
    }
}

fn run_task(target: &Path, task: Task, recipes: &HashMap<String, Recipe>, rootdir: &Path) {

    // if needs_compiling {
    //     let recipe = target
    //         .extension()
    //         .or(Some(OsStr::new("%")))
    //         .and_then(|a| a.to_str())
    //         .and_then(|ext| recipes.get(ext))
    //         .expect(&format!("Failed to lookup recipe for {:?}", target));

    // }
}

fn needs_compiling(file: &Path, dependencies: &[&Path]) -> bool {
    if !std::fs::exists(file).expect(&format!("Unable to determine existance of file {:?}", file)) {
        return true;
    }
    let time = get_modified(file);
    for dep in dependencies {
        if time < get_modified(dep) {
            return true;
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
