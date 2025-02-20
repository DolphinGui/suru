use std::collections::HashMap;

use bld::{build::compile, parser::parse};

fn main() {
    let file = std::fs::read("tasks.bld").expect("File tasks.bld not found");
    let mut context = HashMap::<String, Vec<String>>::default();
    context.insert(
        "LINKFLAGS".into(),
        vec!["-MMD", "-lto", "-O3"]
            .iter()
            .map(|s| s.to_string())
            .collect(),
    );
    let mut bld = Default::default();
    parse(
        std::str::from_utf8(&file).expect("tasks.bld not UTF-8"),
        context,
        &mut bld,
    );
    compile(bld);
}
