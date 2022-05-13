mod parser;
mod transform;

use crate::{parser::BmrParser, transform::BmrTransform};
use std::path::Path;
use swc_common::{
    errors::{ColorConfig, Handler},
    sync::Lrc,
    SourceMap,
};
use swc_ecma_codegen::{
    text_writer::{JsWriter, WriteJs},
    Emitter,
};
use swc_ecma_parser::{
    lexer::Lexer, Capturing, Parser as SWCParser, StringInput, Syntax, TsConfig,
};

fn main() {
    println!("Hello, Boomer!");
    let source_map: Lrc<SourceMap> = Default::default();
    let handler =
        Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(source_map.clone()));

    let source_file = source_map
        .load_file(Path::new("./test_data/input.js"))
        .expect("failed to read file");

    let lexer = Lexer::new(
        Syntax::Typescript(TsConfig {
            tsx: true,
            ..Default::default()
        }),
        Default::default(),
        StringInput::from(&*source_file),
        None,
    );

    let mut swc_parser = SWCParser::new_from(Capturing::new(lexer));

    for e in swc_parser.take_errors() {
        e.into_diagnostic(&handler).emit();
    }

    let module = swc_parser
        .parse_module()
        .map_err(|e| e.into_diagnostic(&handler).emit())
        .expect("failed to parse your boomer file 😞");

    let mut bmr_parser = BmrParser::default();

    let result = bmr_parser
        .parse(module)
        .expect("failed to parse your boomer file 😞");

    for e in bmr_parser.take_errors() {
        e.into_diagnostic(&handler).emit();
    }

    let transformed = BmrTransform::transform(result);

    let mut buf = vec![];

    let wr: Box<dyn WriteJs> = Box::new(JsWriter::new(source_map.clone(), "\n", &mut buf, None));

    Emitter {
        cfg: swc_ecma_codegen::Config { minify: false },
        cm: source_map.clone(),
        comments: None,
        wr,
    }
    .emit_module(&transformed)
    .unwrap();

    std::fs::write(
        "./test_data/output.js",
        String::from_utf8_lossy(&buf).to_string(),
    )
    .expect("failed to write file");
}
