use std::{borrow::Cow, collections::HashMap};

use swc_atoms::JsWord;
use swc_common::{
    errors::{DiagnosticBuilder, Handler},
    Span, Spanned,
};
use swc_ecma_ast::{
    BlockStmt, Expr, FnDecl, JSXElement, LabeledStmt, Module, ModuleDecl, ModuleItem, Stmt,
};

use swc_ecma_visit::{Visit, VisitWith};

#[derive(Debug, Clone, PartialEq)]
pub struct ParserError {
    error: Box<(Span, SyntaxError)>,
}

impl Spanned for ParserError {
    fn span(&self) -> Span {
        (*self.error).0
    }
}

impl ParserError {
    #[cold]
    pub(crate) fn new(span: Span, error: SyntaxError) -> Self {
        Self {
            error: Box::new((span, error)),
        }
    }
    pub fn into_kind(self) -> SyntaxError {
        self.error.1
    }

    #[cold]
    #[inline(never)]
    pub fn into_diagnostic(self, handler: &Handler) -> DiagnosticBuilder {
        let span = self.span();

        let kind = self.into_kind();
        let msg = kind.msg();

        let mut db = handler.struct_err(&msg);
        db.set_span(span);

        db
    }
}

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum SyntaxError {
    UnexpectedLabeledStatement(JsWord),
    LabeledServerIsNotBlock,
    MoreThanOneJSXRoot,
    UnexpectedTopLevelStatement,
    ServerFunctionRedeclared(JsWord),
}

impl SyntaxError {
    #[cold]
    #[inline(never)]
    pub fn msg(&self) -> Cow<'static, str> {
        match self {
            SyntaxError::UnexpectedLabeledStatement(word) => {
                format!("{} is not a valid labeled block", word).into()
            }
            SyntaxError::LabeledServerIsNotBlock => {
                "the server label is not for a block statement".into()
            }
            SyntaxError::MoreThanOneJSXRoot => "only one JSX root permitted per file".into(),
            SyntaxError::UnexpectedTopLevelStatement => "unexpected top level statement".into(),
            SyntaxError::ServerFunctionRedeclared(word) => format!(
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
    _function_declarations: HashMap<JsWord, FunctionDeclaration>,
}

#[derive(Debug)]
pub struct ParseResult {
    pub declarations: Vec<ModuleDecl>,
    pub server: Option<ServerBlock>,
    pub client: Option<LabeledStmt>,
    pub jsx: JSXElement,
}

#[derive(Default)]
pub struct BmrParser {
    errors: Vec<ParserError>,
}

impl BmrParser {
    pub fn parse(&mut self, module: Module) -> anyhow::Result<ParseResult> {
        let mut declarations = vec![];
        let mut server: Option<ServerBlock> = None;
        let mut client: Option<LabeledStmt> = None;
        let mut jsx: Option<JSXElement> = None;

        for module_item in module.body {
            match module_item {
                ModuleItem::ModuleDecl(d) => declarations.push(d),
                ModuleItem::Stmt(s) => match s {
                    Stmt::Labeled(l) => match &*l.label.sym {
                        "server" => {
                            if let Stmt::Block(block) = *l.body {
                                let mut visitor = ServerVisitor::new();

                                visitor.visit_block_stmt(&block);

                                self.errors.append(visitor.take_errors().as_mut());

                                server = Some(ServerBlock {
                                    block: Box::new(block),
                                    _function_declarations: visitor.function_declarations,
                                });
                            } else {
                                self.emit_error(l.span, SyntaxError::LabeledServerIsNotBlock);
                            }
                        }
                        "client" => client = Some(l),
                        _ => {
                            self.emit_error(
                                l.label.span,
                                SyntaxError::UnexpectedLabeledStatement(l.label.sym),
                            );
                        }
                    },
                    Stmt::Expr(e) => {
                        if let Expr::JSXElement(jsx_el) = *e.expr {
                            if jsx.is_some() {
                                self.emit_error(e.span, SyntaxError::MoreThanOneJSXRoot);
                            } else {
                                jsx = Some(*jsx_el);
                            }
                        }
                    }
                    _ => self.emit_error(s.span(), SyntaxError::UnexpectedTopLevelStatement),
                },
            }
        }

        let jsx = jsx.expect("Missing JSX Element");

        let result = ParseResult {
            declarations,
            server,
            client,
            jsx,
        };

        Ok(result)
    }

    #[cold]
    #[inline(never)]
    fn emit_error(&mut self, span: Span, error: SyntaxError) {
        self.errors.push(ParserError::new(span, error));
    }

    pub fn take_errors(&self) -> Vec<ParserError> {
        self.errors.to_owned()
    }
}

#[derive(Debug, Default)]
struct ServerVisitor {
    function_declarations: HashMap<JsWord, FunctionDeclaration>,
    errors: Vec<ParserError>,
}

impl ServerVisitor {
    fn new() -> Self {
        ServerVisitor::default()
    }

    fn take_errors(&self) -> Vec<ParserError> {
        self.errors.to_owned()
    }
}

impl Visit for ServerVisitor {
    fn visit_block_stmt(&mut self, block: &BlockStmt) {
        block.visit_children_with(self)
    }

    fn visit_fn_decl(&mut self, fn_decl: &FnDecl) {
        if self.function_declarations.contains_key(&fn_decl.ident.sym) {
            self.errors.push(ParserError::new(
                fn_decl.span(),
                SyntaxError::ServerFunctionRedeclared(fn_decl.ident.sym.to_owned()),
            ))
        } else {
            let function_declaration = FunctionDeclaration {
                is_async: fn_decl.function.is_async,
            };

            self.function_declarations
                .insert(fn_decl.ident.sym.to_owned(), function_declaration);
        }
    }
}
