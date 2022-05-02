use std::{borrow::Cow, path::Path};

use swc_atoms::JsWord;
use swc_common::{errors::Handler, sync::Lrc, SourceMap, Span, Spanned};
use swc_ecma_ast::{Expr, JSXElement, LabeledStmt, ModuleDecl, ModuleItem, Stmt};
use swc_ecma_parser::{
    lexer::Lexer, Capturing, Parser as SWCParser, StringInput, Syntax, TsConfig,
};

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Error {
    UnexpectedLabeledStatement(JsWord),
    MoreThanOneJSXRoot,
    UnexpectedTopLevelStatement,
}

impl Error {
    #[cold]
    #[inline(never)]
    pub fn msg(&self) -> Cow<'static, str> {
        match self {
            Error::UnexpectedLabeledStatement(word) => {
                format!("{} is not a valid labeled block", word).into()
            }
            Error::MoreThanOneJSXRoot => "only one JSX root permitted per file".into(),
            Error::UnexpectedTopLevelStatement => "unexpected top level statement".into(),
        }
    }
}

#[derive(Debug, Default)]
pub struct ParseResult {
    pub declarations: Vec<ModuleDecl>,
    pub server: Option<LabeledStmt>,
    pub client: Option<LabeledStmt>,
    pub jsx: Option<Box<JSXElement>>,
}

pub struct Parser<'a> {
    source_map: Lrc<SourceMap>,
    handler: &'a Handler,
}

impl<'a> Parser<'a> {
    pub fn new(source_map: Lrc<SourceMap>, handler: &'a Handler) -> Self {
        Parser {
            source_map,
            handler,
        }
    }

    pub fn parse(&mut self, path: &Path) -> anyhow::Result<ParseResult> {
        let source_file = self.source_map.load_file(path)?;

        let lexer = Lexer::new(
            Syntax::Typescript(TsConfig {
                tsx: true,
                ..Default::default()
            }),
            Default::default(),
            StringInput::from(&*source_file),
            None,
        );

        let mut parser = SWCParser::new_from(Capturing::new(lexer));

        for e in parser.take_errors() {
            e.into_diagnostic(&self.handler).emit();
        }

        let ast = parser
            .parse_module()
            .map_err(|e| e.into_diagnostic(&self.handler).emit())
            .expect("failed to parse your boomer file 😞");

        let mut result = ParseResult::default();

        for module_item in ast.body {
            match module_item {
                ModuleItem::ModuleDecl(d) => result.declarations.push(d),
                ModuleItem::Stmt(s) => match s {
                    Stmt::Labeled(l) => match &*l.label.sym {
                        "server" => result.server = Some(l),
                        "client" => result.client = Some(l),
                        _ => {
                            self.emit_error(
                                l.label.span,
                                Error::UnexpectedLabeledStatement(l.label.sym),
                            );
                        }
                    },
                    Stmt::Expr(e) => {
                        if let Expr::JSXElement(jsx) = *e.expr {
                            if result.jsx.is_some() {
                                self.emit_error(e.span, Error::MoreThanOneJSXRoot);
                            }

                            result.jsx = Some(jsx);
                        }
                    }
                    _ => self.emit_error(s.span(), Error::UnexpectedTopLevelStatement),
                },
            }
        }

        Ok(result)
    }

    fn emit_error(&mut self, span: Span, error: Error) {
        let mut db = self.handler.struct_err(&error.msg());
        db.set_span(span);
        db.emit();
    }
}
