use std::{
    collections::HashMap,
    fs::{DirEntry, Metadata},
    path::{Path, PathBuf},
    str::FromStr,
};

use paris::error;
use yaru::{build::compile, parser::parse};

fn main() {
    let cwd = &std::env::current_dir().expect("Unable to open current working directory");
    let file = find_file(cwd);

    let tasks = std::fs::read(&file).expect("Could not read task file");
    let mut context = HashMap::<String, Vec<String>>::default();
    let prologue = include_str!("prologue.bld");
    let mut bld = Default::default();

    parse(prologue, &mut context, &mut bld);

    let mut depfiles = Vec::new();
    search_dependencies(&cwd, &mut depfiles);

    for depfile in depfiles {
        let tasks = std::fs::read(&depfile);
        match tasks {
            Err(e) => {
                error!(
                    "Unable to read depfile {:?} due to: {}",
                    depfile.file_name(),
                    e
                );
            }
            Ok(depfile) => {
                parse(&preprocess(depfile), &mut context, &mut bld);
            }
        }
    }

    parse(&preprocess(tasks), &mut context, &mut bld);

    compile(
        bld,
        cwd,
        file.parent().expect("Unable to open parent of task file"),
    );
}

fn find_file(search_root: &Path) -> PathBuf {
    for parent in search_root.ancestors() {
        match parent.read_dir() {
            Ok(d) => {
                for file in d {
                    match file {
                        Ok(f) => {
                            if f.file_name() == "tasks.bld" {
                                return f.path();
                            }
                        }
                        Err(e) => {
                            error!(
                                "Unable to read file {} due to {}",
                                parent
                                    .as_os_str()
                                    .to_str()
                                    .expect("Unable to parse UTF-8 error message"),
                                e
                            )
                        }
                    }
                }
            }
            Err(e) => {
                // Don't actually panic because this usually isn't a fatal error
                error!(
                    "Unable to search directory {} for build files due to {}",
                    parent
                        .as_os_str()
                        .to_str()
                        .expect("Unable to parse UTF-8 error message"),
                    e
                )
            }
        }
    }
    error!("Unable to find tasks.bld in any parent directories");
    panic!();
}

fn search_dependencies(search_root: &Path, out: &mut Vec<PathBuf>) {
    for file in search_root
        .read_dir()
        .expect("Cannot search files in build root")
        .filter_map(|d| d.ok())
    {
        if file.file_type().is_ok_and(|t| t.is_dir()) {
            search_dependencies(&file.path(), out);
        } else if file
            .file_name()
            .to_str()
            .and_then(|f| Some(f.ends_with(".d")))
            .unwrap_or(false)
        {
            out.push(file.path());
        }
    }
}

fn preprocess(file: Vec<u8>) -> String {
    let file = String::from_utf8(file).expect("Build file is not utf-8");
    file.replace("\\\n", " ")
}
