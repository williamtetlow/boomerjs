mod codegen;
mod parser;

use std::path::Path;

use swc_common::{
    errors::{ColorConfig, Handler},
    sync::Lrc,
    SourceMap,
};

use crate::{
    codegen::{Emitter, SharedBuffer},
    parser::Parser,
};

fn main() {
    println!("Hello, Boomer!");
    let source_map: Lrc<SourceMap> = Default::default();
    let handler =
        Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(source_map.clone()));

    let mut parser = Parser::new(source_map.clone(), &handler);
    let result = parser
        .parse(Path::new("./input.js"))
        .expect("failed during parsing");

    let buf = SharedBuffer::default();

    let mut emitter = Emitter::new(source_map.clone(), buf.clone(), buf.clone());

    emitter
        .emit_bmr_stmts(result)
        .expect("failed during codegen");

    let out = &*buf.0.lock().unwrap();

    std::fs::write("./out.js", out).expect("failed to write file");
}
