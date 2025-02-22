use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    str::FromStr,
};

use clap::Parser;
use indicatif::MultiProgress;
use indicatif_log_bridge::LogWrapper;
use log::error;
use suru::{build::compile, parser::parse};

#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    build_dir: Option<String>,
    #[arg(short, long)]
    source_dir: Option<String>,
}

fn init_logging() -> MultiProgress {
    let logger =
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).build();

    let level = logger.filter();
    let mp = MultiProgress::new();
    if let Err(e) = LogWrapper::new(mp.clone(), logger).try_init() {
        panic!("Could not thing {}", e);
    }
    log::set_max_level(level);
    mp
}

fn main() {
    let args = Args::parse();

    let mp = init_logging();

    let build_root = args
        .build_dir
        .and_then(|s| PathBuf::from_str(&s).ok())
        .or(std::env::current_dir().ok())
        .expect("Unable to open current working directory");

    let search_root = args
        .source_dir
        .and_then(|s| match PathBuf::from_str(&s) {
            Ok(p) => Some(p),
            Err(e) => {
                error!("Unable to open specified search root: {}", e);
                None
            }
        })
        .unwrap_or_else(|| build_root.clone());

    let taskfile = find_file(&search_root);

    let tasks = std::fs::read(&taskfile).expect("Could not read task file");
    let mut context = HashMap::<String, Vec<String>>::default();
    let prologue = include_str!("prologue.su");
    let mut buildstate = Default::default();

    parse(prologue, &mut context, &mut buildstate);

    let mut depfiles = Vec::new();
    search_dependencies(&build_root, &mut depfiles);

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
                parse(&preprocess(depfile), &mut context, &mut buildstate);
            }
        }
    }

    parse(&preprocess(tasks), &mut context, &mut buildstate);

    compile(
        buildstate,
        &build_root,
        taskfile
            .parent()
            .expect("Unable to open parent of task file"),
        mp,
    );
}

fn find_file(search_root: &Path) -> PathBuf {
    for parent in search_root.ancestors() {
        match parent.read_dir() {
            Ok(d) => {
                for file in d {
                    match file {
                        Ok(f) => {
                            if f.file_name() == "tasks.su" {
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
    error!("Unable to find tasks.su in any parent directories");
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
