use std::{
    ffi::OsString,
    fs::File,
    path::{Path, PathBuf},
};

use crossbeam::queue::SegQueue;
use log::error;
use serde::Serialize;

pub struct HookResult {
    compile_cmd: CompileCommand,
}

#[derive(Serialize)]
struct CompileCommand {
    directory: String,
    arguments: Vec<String>,
    file: String,
}

pub fn pre_compile(
    out: &SegQueue<HookResult>,
    cmd: &[OsString],
    deps: &[PathBuf],
    target: &Path,
    sourcedir: &Path,
) {
    if cmd.len() < 2 {
        return;
    }

    if let [exe, ..] = cmd {
        let exe = exe.to_string_lossy();
        // a heuristic used to tell if it's a c/c++ compiler or not.
        if !exe.contains("gcc") && !exe.contains("g++") && !exe.contains("clang") {
            return;
        }
    }

    // heuristically tell if it's a c/cpp file.
    if deps
        .iter()
        .all(|d| !d.extension().unwrap().to_string_lossy().contains("c"))
    {
        return;
    }

    out.push(HookResult {
        compile_cmd: CompileCommand {
            directory: sourcedir
                .as_os_str()
                .to_str()
                .expect("Invalid unicode in path")
                .to_owned(),
            arguments: cmd
                .iter()
                .map(|s| s.to_str().expect("Invalid unicode in command").to_owned())
                .collect(),
            file: target
                .as_os_str()
                .to_str()
                .expect("Invalid unicode in target path")
                .to_owned(),
        },
    });
}

pub fn post_compile(input: &SegQueue<HookResult>, workdir: &Path) {
    let mut compiledb = Vec::new();
    compiledb.reserve(input.len());
    while let Some(result) = input.pop() {
        compiledb.push(result.compile_cmd);
    }
    let db_file = File::create(workdir.join("compile_commands.json"));
    match db_file {
        Err(e) => {
            error!("Unable to open compile_commands.json: {}", e);
            return;
        }
        Ok(f) => {
            if let Err(e) = serde_json::to_writer_pretty(f, &compiledb) {
                error!("Unable to write to compile_commands.json: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod testing {
    use crate::util::make_svec;

    use super::*;

    #[test]
    fn test_compiledb() {
        let db = vec![CompileCommand {
            directory: "/home/user/llvm/build".into(),
            arguments: make_svec(&[
                "/usr/bin/clang++",
                "-Irelative",
                "-DSOMEDEF=With spaces, quotes and \\-es.",
                "-c",
                "-o",
                "file.o",
                "file.cc",
            ]),
            file: "file.cc".into(),
        }];
        let string = serde_json::to_string_pretty(&db);
        assert!(string.is_ok());
        println!("compiledb:\n{}", string.unwrap());
    }
}
