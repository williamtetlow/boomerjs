use std::{
    fmt::{Result, Write},
    path::Path,
};

use auto_impl::auto_impl;
use swc_common::{
    errors::{ColorConfig, Handler},
    sync::Lrc,
    SourceMap,
};
use swc_ecma_ast::{
    Expr, Ident, JSXClosingElement, JSXElement, JSXElementName, JSXOpeningElement, JSXText,
    ModuleItem, Stmt,
};
use swc_ecma_parser::{lexer::Lexer, Capturing, Parser, StringInput, Syntax, TsConfig};
use swc_ecma_visit::{
    as_folder, noop_visit_mut_type, Fold, FoldWith, Visit, VisitMut, VisitMutWith, VisitWith,
};

enum TopLevelStmt {
    Server,
    Client,
    JSX,
}

struct Boomer;

impl Boomer {
    pub fn new() -> Self {
        Boomer {}
    }

    fn visit_mut_top_level_stmt(&mut self, stmt: &mut Stmt) {
        match stmt {
            Stmt::Labeled(l) => {
                if &*l.label.sym == "server" || &*l.label.sym == "client" {
                    return ();
                }

                panic!("Unexpected top level labeled block - only client/server blocks allowed");
            }
            Stmt::Expr(e) => {
                if let Expr::JSXElement(jsx_element) = &mut *e.expr {
                    JSXVisitor.visit_mut_jsx_root(jsx_element);
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

impl VisitMut for Boomer {
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

struct JSXVisitor;

impl JSXVisitor {
    fn visit_mut_jsx_root(&mut self, el: &mut JSXElement) {
        el.visit_mut_with(self);
    }
}

impl VisitMut for JSXVisitor {
    noop_visit_mut_type!();

    fn visit_mut_jsx_element(&mut self, el: &mut JSXElement) {
        // println!("{:?}", el);
    }
}

#[auto_impl(&mut, Box)]
trait JSXWriter {
    fn write_space(&mut self) -> Result;

    fn write_newline(&mut self) -> Result;

    fn write_raw(&mut self, text: &str) -> Result;

    fn write_str(&mut self, s: &str) -> Result;
}

struct BasicJSXWriter<W>
where
    W: Write,
{
    writer: W,
}

impl<W> BasicJSXWriter<W>
where
    W: Write,
{
    pub fn new(writer: W) -> Self {
        BasicJSXWriter { writer }
    }

    fn write(&mut self, data: &str) -> Result {
        if data.is_empty() {
            return Ok(());
        }

        self.raw_write(data)?;

        Ok(())
    }

    fn raw_write(&mut self, data: &str) -> Result {
        self.writer.write_str(data)?;

        Ok(())
    }
}

impl<W> JSXWriter for BasicJSXWriter<W>
where
    W: Write,
{
    fn write_space(&mut self) -> Result {
        self.write_raw(" ")
    }

    fn write_newline(&mut self) -> Result {
        self.raw_write("\n")?;
        Ok(())
    }

    fn write_raw(&mut self, text: &str) -> Result {
        debug_assert!(
            !text.contains('\n'),
            "write_raw should not contains new lines, got '{}'",
            text,
        );

        self.write(text)?;

        Ok(())
    }

    fn write_str(&mut self, s: &str) -> Result {
        if s.is_empty() {
            return Ok(());
        }

        let mut lines = s.split('\n').peekable();

        while let Some(line) = lines.next() {
            self.raw_write(line)?;

            if lines.peek().is_some() {
                self.raw_write("\n")?;
            }
        }

        Ok(())
    }
}

struct CodeGenerator<W>
where
    W: JSXWriter,
{
    writer: W,
}

impl<W> CodeGenerator<W>
where
    W: JSXWriter,
{
    pub fn new(writer: W) -> Self {
        CodeGenerator { writer }
    }

    fn emit_jsx(&mut self, root: &JSXElement) -> Result {
        self.writer.write_raw("export const $$Component = [`")?;
        root.visit_with(self);
        self.writer.write_raw("`]")?;
        self.writer.write_newline()?;
        Ok(())
    }
}

impl<W> Visit for CodeGenerator<W>
where
    W: JSXWriter,
{
    fn visit_jsx_element(&mut self, node: &JSXElement) {
        node.visit_children_with(self);
    }

    fn visit_jsx_opening_element(&mut self, node: &JSXOpeningElement) {
        self.writer.write_raw("<");
        node.name.visit_with(self);

        if node.self_closing {
            self.writer.write_raw("/");
        }
        self.writer.write_raw(">");
    }

    fn visit_jsx_closing_element(&mut self, node: &JSXClosingElement) {
        self.writer.write_raw("</");
        node.name.visit_with(self);
        self.writer.write_raw(">");
    }

    fn visit_jsx_element_name(&mut self, node: &JSXElementName) {
        match *node {
            JSXElementName::Ident(ref n) => n.visit_with(self),
            JSXElementName::JSXMemberExpr(ref n) => n.visit_with(self),
            JSXElementName::JSXNamespacedName(ref n) => n.visit_with(self),
        }
    }

    fn visit_jsx_text(&mut self, node: &JSXText) {
        self.writer.write_str(&*node.value);
    }

    fn visit_ident(&mut self, node: &Ident) {
        self.writer.write_raw(&*node.sym);
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
        .expect("Failed to parse");

    let mut boomer = Boomer::new();

    let mut folder = as_folder(&mut boomer);

    let boomer = folder.fold_module(ast);

    let mut wr = String::new();

    let mut code_generator = CodeGenerator::new(BasicJSXWriter::new(&mut wr));

    let jsx = boomer.body[0]
        .as_stmt()
        .unwrap()
        .as_expr()
        .unwrap()
        .expr
        .as_jsx_element()
        .unwrap();

    code_generator.emit_jsx(&jsx).expect("Failed to emit");

    std::fs::write("./out.js", wr).expect("Failed to write file");
}
