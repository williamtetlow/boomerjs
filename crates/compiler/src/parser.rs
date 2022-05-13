use std::{borrow::Cow, collections::HashMap, rc};

use swc_atoms::JsWord;
use swc_common::{errors::Handler, sync::Lrc, SourceFile, SourceMap, Span, Spanned};
use swc_ecma_ast::{
    BlockStmt, Expr, FnDecl, Ident, JSXClosingElement, JSXElement, JSXElementChild, JSXElementName,
    JSXExpr, JSXExprContainer, JSXOpeningElement, JSXText, LabeledStmt, ModuleDecl, ModuleItem,
    Stmt,
};
use swc_ecma_parser::{
    lexer::Lexer, Capturing, Parser as SWCParser, StringInput, Syntax, TsConfig,
};
use swc_ecma_visit::{Visit, VisitWith};

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Error {
    UnexpectedLabeledStatement(JsWord),
    LabeledServerIsNotBlock,
    LabeledClientIsNotBlock,
    MoreThanOneJSXRoot,
    UnexpectedTopLevelStatement,
    ServerFunctionRedeclared(JsWord),
}

impl Error {
    #[cold]
    #[inline(never)]
    pub fn msg(&self) -> Cow<'static, str> {
        match self {
            Error::UnexpectedLabeledStatement(word) => {
                format!("{} is not a valid labeled block", word).into()
            }
            Error::LabeledServerIsNotBlock => {
                "the server label is not for a block statement".into()
            }
            Error::LabeledClientIsNotBlock => {
                "the client label is not for a block statement".into()
            }
            Error::MoreThanOneJSXRoot => "only one JSX root permitted per file".into(),
            Error::UnexpectedTopLevelStatement => "unexpected top level statement".into(),
            Error::ServerFunctionRedeclared(word) => format!(
                "server functions can only be declared once, {} has multiple declarations",
                word
            )
            .into(),
        }
    }
}

#[derive(Debug)]
pub struct FunctionDeclaration {
    pub is_async: bool,
}

#[derive(Debug)]
pub struct ServerBlock {
    pub block: Box<BlockStmt>,
    function_declarations: HashMap<JsWord, FunctionDeclaration>,
}

impl ServerBlock {
    pub fn contains_function_declaration(&self, ident: &Ident) -> bool {
        self.function_declarations.contains_key(&ident.sym)
    }

    pub fn get_function_declaration(&self, ident: &Ident) -> Option<&FunctionDeclaration> {
        self.function_declarations.get(&ident.sym)
    }
}

#[derive(Debug, Default)]
pub struct ParseResult {
    pub declarations: Vec<ModuleDecl>,
    pub server: Option<ServerBlock>,
    pub client: Option<LabeledStmt>,
    pub jsx: Option<JSXElement>,
}

pub struct Parser<'a> {
    handler: &'a Handler,
}

impl<'a> Parser<'a> {
    pub fn new(handler: &'a Handler) -> Self {
        Parser { handler }
    }

    pub fn parse(&mut self, source_file: rc::Rc<SourceFile>) -> anyhow::Result<ParseResult> {
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
                        "server" => {
                            if let Stmt::Block(block) = *l.body {
                                let mut visitor = ServerVisitor::new();

                                visitor.visit_block_stmt(&block);

                                for e in visitor.take_errors() {
                                    self.emit_error(e.0, e.1);
                                }

                                result.server = Some(ServerBlock {
                                    block: Box::new(block),
                                    function_declarations: visitor.function_declarations,
                                });
                            } else {
                                self.emit_error(l.span, Error::LabeledServerIsNotBlock);
                            }
                        }
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
                            } else {
                                result.jsx = Some(*jsx);
                            }
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

#[derive(Debug, Default)]
struct ServerVisitor {
    function_declarations: HashMap<JsWord, FunctionDeclaration>,
    errors: Vec<Box<(Span, Error)>>,
}

impl ServerVisitor {
    fn new() -> Self {
        ServerVisitor::default()
    }

    fn take_errors(&self) -> Vec<Box<(Span, Error)>> {
        self.errors.to_owned()
    }
}

impl Visit for ServerVisitor {
    fn visit_block_stmt(&mut self, block: &BlockStmt) {
        block.visit_children_with(self)
    }

    fn visit_fn_decl(&mut self, fn_decl: &FnDecl) {
        if self.function_declarations.contains_key(&fn_decl.ident.sym) {
            self.errors.push(Box::new((
                fn_decl.span(),
                Error::ServerFunctionRedeclared(fn_decl.ident.sym.to_owned()),
            )))
        } else {
            let function_declaration = FunctionDeclaration {
                is_async: fn_decl.function.is_async,
            };

            self.function_declarations
                .insert(fn_decl.ident.sym.to_owned(), function_declaration);
        }
    }
}
