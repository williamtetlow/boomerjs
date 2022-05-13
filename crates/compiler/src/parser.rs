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
pub struct BoomerServerBlock {
    pub block: Box<BlockStmt>,
    function_declarations: HashMap<JsWord, FunctionDeclaration>,
}

impl BoomerServerBlock {
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
    pub server: Option<BoomerServerBlock>,
    pub client: Option<LabeledStmt>,
    pub jsx: Option<JSXElement>,
    pub markup: Option<Node>,
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

                                result.server = Some(BoomerServerBlock {
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
                                // result.markup =
                                //     Some(BoomerMarkupTransformer.parse_jsx_element(*jsx));
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

#[derive(Debug)]
struct BoomerMarkup {
    root: Node,
}

#[derive(Debug)]
pub enum Node {
    HTMLElement(HTMLElement),
    Component(Component),
    NotHandledYet,
}

#[derive(Debug)]
pub enum Child {
    Text(JSXText),
    Expression(Expression),
    HTMLElement(Box<HTMLElement>),
    Component(Box<Component>),
    NotHandledYet,
}

#[derive(Debug, Spanned)]
pub struct HTMLElement {
    pub span: Span,
    pub opening: JSXOpeningElement,
    pub children: Vec<Child>,
    pub closing: Option<JSXClosingElement>,
}

#[derive(Debug, Spanned)]
pub struct Component {
    pub span: Span,
    pub opening: JSXOpeningElement,
    pub children: Vec<Child>,
    pub closing: Option<JSXClosingElement>,
}

#[derive(Debug, Spanned)]
pub struct Expression {
    pub span: Span,
    pub expr: JSXExpr,
}

/*
    {get()}
*/

struct BoomerMarkupTransformer;

impl BoomerMarkupTransformer {}

trait JSXElementBoomerType {
    fn is_component(&self) -> bool;
    fn is_html(&self) -> bool;
}

impl JSXElementBoomerType for JSXElement {
    fn is_component(&self) -> bool {
        !self.is_html()
    }

    fn is_html(&self) -> bool {
        match &self.opening.name {
            JSXElementName::Ident(id) => id
                .sym
                .chars()
                .nth(0)
                .map_or(false, |first_char| first_char.is_lowercase()),
            _ => false,
        }
    }
}

impl BoomerMarkupTransformer {
    pub fn parse_boomer_markup(&mut self, root: JSXElement) -> BoomerMarkup {
        let parsed_root = self.parse_jsx_element(root);

        BoomerMarkup { root: parsed_root }
    }

    fn parse_jsx_element(&mut self, n: JSXElement) -> Node {
        if n.is_html() {
            Node::HTMLElement(HTMLElement {
                span: n.span,
                opening: n.opening,
                children: self.parse_jsx_element_children(n.children),
                closing: n.closing,
            })
        } else {
            Node::Component(Component {
                span: n.span,
                opening: n.opening,
                children: self.parse_jsx_element_children(n.children),
                closing: n.closing,
            })
        }
    }

    fn parse_jsx_expr_container(&mut self, n: JSXExprContainer) -> Child {
        Child::Expression(Expression {
            span: n.span,
            expr: n.expr,
        })
    }

    fn parse_jsx_element_children(&mut self, n: Vec<JSXElementChild>) -> Vec<Child> {
        n.into_iter()
            .map(|child| self.parse_jsx_element_child(child))
            .collect()
    }

    fn parse_jsx_element_child(&mut self, n: JSXElementChild) -> Child {
        match n {
            JSXElementChild::JSXText(text) => Child::Text(text),
            JSXElementChild::JSXElement(el) => match self.parse_jsx_element(*el) {
                Node::Component(c) => Child::Component(Box::new(c)),
                Node::HTMLElement(h) => Child::HTMLElement(Box::new(h)),
                _ => Child::NotHandledYet,
            },
            JSXElementChild::JSXExprContainer(expr) => self.parse_jsx_expr_container(expr),
            _ => Child::NotHandledYet,
        }
    }
}

#[cfg(test)]
mod test {
    use std::path::Path;

    use swc_common::{
        errors::{ColorConfig, Handler},
        sync::Lrc,
        FileName, SourceMap, DUMMY_SP,
    };
    use swc_ecma_ast::JSXText;

    use crate::parser::{Child, Component, Node};

    use super::{BoomerMarkup, BoomerMarkupTransformer, HTMLElement, Parser};

    macro_rules! test {
        ($input:expr,$(|)? $( $pattern:pat_param )|+ $( if $guard: expr )? $(,)?) => {
            let source_map: Lrc<SourceMap> = Default::default();
            let handler =
                Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(source_map.clone()));

            let mut parser = Parser::new(source_map.clone(), &handler);

            let source_file =
                source_map.new_source_file(FileName::Custom("./test.js".into()), $input.to_owned());

            let result = parser.parse(source_file).expect("failed to parse");

            let jsx = result.jsx.unwrap();

            let mut transformer = BoomerMarkupTransformer;

            let transformed = transformer.parse_boomer_markup(jsx);

            assert!(matches!(transformed.root, $( $pattern )|+ $( if $guard )?))
        };
    }

    #[test]
    fn it_parses_html_element() {
        test!(
            "<h1>Hello World</h1>",
            Node::HTMLElement(HTMLElement { children, .. }) if matches!(children.as_slice(), [Child::Text(JSXText { value, .. })] if &*value == "Hello World")
        );
    }

    #[test]
    fn it_parses_component() {
        test!("<Component>Hello World</Component>", Node::Component(Component { children, .. }) if matches!(children.as_slice(), [Child::Text(JSXText { value, .. })] if &*value == "Hello World"));
    }

    #[test]
    fn it_parses_nested_elements() {
        test!("<nav><NavItem>Home</NavItem><NavItem>About</NavItem></nav>", Node::HTMLElement(HTMLElement { children, .. }) if matches!(children.as_slice(), [Child::Component(_), Child::Component(_)]));
    }

    #[test]
    fn it_parses_expressions() {
        test!("<div>{getData()}</div>", Node::HTMLElement(HTMLElement { children, .. }) if matches!(children.as_slice(), [Child::Expression(_)]));
    }
}
