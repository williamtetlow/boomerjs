use std::{path::Path, rc::Rc};

use swc_common::{
    errors::{ColorConfig, Handler},
    sync::Lrc,
    SourceMap,
};
use swc_ecma_ast::{Expr, ImportDecl, ModuleItem, Stmt};
use swc_ecma_parser::{lexer::Lexer, Capturing, Parser, StringInput, Syntax, TsConfig};
use swc_ecma_visit::{as_folder, noop_visit_mut_type, Fold, VisitMut};

struct BoomerVisitor;

impl BoomerVisitor {
    fn visit_mut_top_level_stmt(&mut self, stmt: &mut Stmt) {
        match stmt {
            Stmt::Labeled(l) => {
                if &*l.label.sym == "server" || &*l.label.sym == "client" {
                    return ();
                }

                panic!("Unexpected top level labeled block - only client/server blocks allowed");
            }
            Stmt::Expr(e) => {
                if let Expr::JSXElement(jsx_element) = &*e.expr {
                    return ();
                }

                panic!("Unexpected top level expression - only JSX expressions are allowed");
            }
            _ => {
                panic!("Unexpected top level statement - only JSX or client/server blocks allowed")
            }
        }
    }
}

impl VisitMut for BoomerVisitor {
    noop_visit_mut_type!();

    fn visit_mut_module_items(&mut self, items: &mut Vec<ModuleItem>) {
        for item in items {
            match item {
                ModuleItem::ModuleDecl(_) => (),
                ModuleItem::Stmt(stmt) => self.visit_mut_top_level_stmt(stmt),
            }
        }
    }
}

fn main() {
    println!("Hello, Boomer!");

    let cm: Lrc<SourceMap> = Default::default();
    let handler = Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(cm.clone()));

    let fm = cm
        .load_file(Path::new("./input.js"))
        .expect("failed to load input.js");

    let lexer = Lexer::new(
        Syntax::Typescript(TsConfig {
            tsx: true,
            ..Default::default()
        }),
        Default::default(),
        StringInput::from(&*fm),
        None,
    );

    let capturing = Capturing::new(lexer);

    let mut parser = Parser::new_from(capturing);

    for e in parser.take_errors() {
        e.into_diagnostic(&handler).emit();
    }

    let ast = parser
        .parse_module()
        .map_err(|e| e.into_diagnostic(&handler).emit())
        .expect("Failed to parse script.");

    let mut folder = as_folder(BoomerVisitor);

    println!("{:#?}", &ast);

    folder.fold_module(ast);
}
