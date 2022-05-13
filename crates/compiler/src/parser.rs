use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
};

use swc_atoms::JsWord;
use swc_common::{
    errors::{DiagnosticBuilder, Handler},
    Span, Spanned,
};
use swc_ecma_ast::{
    BlockStmt, Callee, Expr, FnDecl, JSXElement, Module, ModuleDecl, ModuleItem, Pat, Stmt,
    VarDeclarator,
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
    LabeledClientIsNotBlock,
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
            SyntaxError::LabeledClientIsNotBlock => {
                "the client label is not for a block statement".into()
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
    pub block: BlockStmt,
    _function_declarations: HashMap<JsWord, FunctionDeclaration>,
}

#[derive(Debug)]
pub struct ClientBlock {
    pub block: BlockStmt,
    pub use_state: UseStateDeclarations,
}

#[derive(Debug)]
pub struct ParseResult {
    pub declarations: Vec<ModuleDecl>,
    pub server: Option<ServerBlock>,
    pub client: Option<ClientBlock>,
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
        let mut client: Option<ClientBlock> = None;
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
                                    block,
                                    _function_declarations: visitor.function_declarations,
                                });
                            } else {
                                self.emit_error(l.span, SyntaxError::LabeledServerIsNotBlock);
                            }
                        }
                        "client" => {
                            if let Stmt::Block(block) = *l.body {
                                let mut visitor = ClientVisitor::default();

                                visitor.visit_block_stmt(&block);

                                client = Some(ClientBlock {
                                    block,
                                    use_state: visitor.use_state,
                                });
                            } else {
                                self.emit_error(l.span, SyntaxError::LabeledClientIsNotBlock);
                            }
                        }
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

#[derive(Debug, Default)]
pub struct UseStateDeclarations {
    pub get: HashSet<JsWord>,
    pub set: HashSet<JsWord>,
}

#[derive(Default)]
struct ClientVisitor {
    use_state: UseStateDeclarations,
}

impl ClientVisitor {
    fn record_use_state_declaration(&mut self, decl: &VarDeclarator) {
        if let Pat::Array(arr) = &decl.name {
            match arr.elems.len() {
                2 => {
                    let get = arr.elems[0].as_ref();
                    let set = arr.elems[1].as_ref();

                    match (get, set) {
                        (Some(Pat::Ident(get)), Some(Pat::Ident(set))) => {
                            self.use_state.get.insert(get.id.sym.clone());
                            self.use_state.set.insert(set.id.sym.clone());
                        }
                        _ => (), // TODO: throw error
                    }
                }
                _ => (), // TODO: throw error
            }
        }
        // TODO: handle other ways of initialising (e.g. const a = useState(0))
    }
}

impl Visit for ClientVisitor {
    fn visit_var_declarator(&mut self, decl: &VarDeclarator) {
        if let Some(init) = &decl.init {
            if let Expr::Call(call) = &**init {
                if let Callee::Expr(e) = &call.callee {
                    if let Expr::Ident(id) = &**e {
                        if &*id.sym == "useState" {
                            self.record_use_state_declaration(decl);
                        }
                    }
                }
            }
        }
    }
}
