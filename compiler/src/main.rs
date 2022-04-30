use std::{
    cell::RefCell,
    fmt::Result,
    io::{Read, Write},
    path::Path,
    rc::Rc,
    sync::{Arc, Mutex},
};

use auto_impl::auto_impl;
use swc_common::{
    errors::{ColorConfig, Handler},
    sync::Lrc,
    FilePathMapping, SourceMap,
};
use swc_ecma_ast::{
    Expr, ExprStmt, Ident, JSXClosingElement, JSXElement, JSXElementName, JSXExprContainer,
    JSXOpeningElement, JSXText, ModuleItem, Script, Stmt,
};
use swc_ecma_codegen::{
    text_writer::{JsWriter, WriteJs},
    Emitter, Node,
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
        self.writer.write(data.as_bytes());

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

struct CodeGenerator<'a, WJSX, WJS>
where
    WJSX: JSXWriter,
    WJS: WriteJs,
{
    writer: WJSX,
    js_writer: &'a mut Emitter<'a, WJS>,
}

impl<'a, WJSX, WJS> CodeGenerator<'a, WJSX, WJS>
where
    WJSX: JSXWriter,
    WJS: WriteJs,
{
    pub fn new(writer: WJSX, js_writer: &'a mut Emitter<'a, WJS>) -> Self {
        CodeGenerator { writer, js_writer }
    }

    fn emit_jsx(&mut self, root: &JSXElement) -> Result {
        self.writer.write_raw("export const $$Component = [`")?;
        root.visit_with(self);
        self.writer.write_raw("`]")?;
        self.writer.write_newline()?;
        Ok(())
    }
}

impl<'a, WJSX, WJS> Visit for CodeGenerator<'a, WJSX, WJS>
where
    WJSX: JSXWriter,
    WJS: WriteJs,
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

    fn visit_jsx_expr_container(&mut self, node: &JSXExprContainer) {
        self.writer.write_raw("`,");
        node.expr.emit_with(&mut self.js_writer);
        self.writer.write_raw(",`");
    }

    fn visit_jsx_text(&mut self, node: &JSXText) {
        self.writer.write_str(&*node.value);
    }

    fn visit_ident(&mut self, node: &Ident) {
        self.writer.write_raw(&*node.sym);
    }
}

trait EmitBoomer {
    fn emit_stmt(&mut self, stmt: &Stmt);
    fn emit_jsx_expr(&mut self, expr: &Expr);
}

impl<'a, W> EmitBoomer for Emitter<'a, W>
where
    W: WriteJs,
{
    fn emit_stmt(&mut self, stmt: &Stmt) {
        stmt.emit_with(self).unwrap();
    }

    fn emit_jsx_expr(&mut self, expr: &Expr) {
        expr.emit_with(self).unwrap();
    }
}

#[derive(Debug, Default)]
struct SharedBuffer(Arc<Mutex<Vec<u8>>>);

impl Write for SharedBuffer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.0.lock().unwrap().flush()
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

    let cm = Lrc::new(SourceMap::new(FilePathMapping::empty()));

    let server = boomer.body[0]
        .as_stmt()
        .unwrap()
        .as_labeled()
        .unwrap()
        .body
        .as_block()
        .unwrap()
        .stmts
        .to_owned();

    let buf = SharedBuffer::default();

    let mut emitter = Emitter {
        cfg: swc_ecma_codegen::Config { minify: false },
        cm: cm.clone(),
        comments: None,
        wr: JsWriter::new(cm, "\n", SharedBuffer(Arc::clone(&buf.0)), None),
    };

    for stmt in server {
        emitter.emit_stmt(&stmt);
    }

    let mut code_generator = CodeGenerator::new(
        BasicJSXWriter::new(SharedBuffer(Arc::clone(&buf.0))),
        &mut emitter,
    );

    let jsx = boomer.body[1]
        .as_stmt()
        .unwrap()
        .as_expr()
        .unwrap()
        .expr
        .as_jsx_element()
        .unwrap();

    code_generator.emit_jsx(&jsx).expect("Failed to emit");

    std::fs::write("./out.js", (*buf.0.lock().unwrap()).to_owned()).expect("Failed to write file");
}
