use std::collections::HashMap;

use bld::{build::compile, parser::parse};

fn main() {
    let file = std::fs::read("tasks.bld").expect("File tasks.bld not found");
    let mut context = HashMap::<String, Vec<String>>::default();
    let prologue = include_str!("prologue.bld");
    let mut bld = Default::default();

    parse(prologue, &mut context, &mut bld);

    parse(
        std::str::from_utf8(&file).expect("tasks.bld not UTF-8"),
        &mut context,
        &mut bld,
    );
    compile(bld);
}
