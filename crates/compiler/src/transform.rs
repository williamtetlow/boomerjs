use swc_atoms::JsWord;
use swc_common::DUMMY_SP;
use swc_ecma_ast::{
    ArrayLit, BlockStmt, Decl, ExportDecl, Expr, ExprOrSpread, FnDecl, Function, Ident, JSXElement,
    JSXElementName, JSXExpr, Lit, Module, ModuleDecl, ModuleItem, ReturnStmt, Stmt, Str,
};
use swc_ecma_visit::{Visit, VisitWith};

use crate::parser::{ParseResult, ServerBlock};

pub struct BmrTransform;

impl BmrTransform {
    pub fn transform(parse_result: ParseResult) -> Module {
        let mut jsx_transform = JSXTransform::default();
        let markup = jsx_transform.transform(parse_result.jsx);
        let server_stmts = if let Some(server_block) = parse_result.server {
            ServerTransform::transform(server_block)
        } else {
            vec![]
        };

        let mut module_items: Vec<ModuleItem> = vec![];

        for stmt in server_stmts {
            module_items.push(ModuleItem::Stmt(stmt));
        }

        for decl in markup {
            module_items.push(ModuleItem::ModuleDecl(decl));
        }

        Module {
            shebang: None,
            span: DUMMY_SP,
            body: module_items,
        }
    }
}

#[derive(Default)]
struct JSXTransform {
    cur_children: Vec<Option<ExprOrSpread>>,
}

macro_rules! html_open_tag {
    ($tag:expr) => {
        str_lit!(
            JsWord::from(format!(r#""<{}>""#, $tag)),
            JsWord::from(format!("<{}>", $tag))
        )
    };
}

macro_rules! html_close_tag {
    ($tag:expr) => {
        str_lit!(
            JsWord::from(format!(r#""</{}>""#, $tag)),
            JsWord::from(format!("</{}>", $tag))
        )
    };
}

macro_rules! str_lit {
    ($val:expr, $raw:expr) => {
        Some(ExprOrSpread {
            spread: None,
            expr: Box::new(Expr::Lit(Lit::Str(Str {
                span: DUMMY_SP,
                raw: Some($val),
                value: $raw,
            }))),
        })
    };
}

macro_rules! array_lit {
    ($elems:expr) => {
        Some(ExprOrSpread {
            spread: None,
            expr: Box::new(Expr::Array(ArrayLit {
                span: DUMMY_SP,
                elems: $elems,
            })),
        })
    };
}

impl JSXTransform {
    pub fn transform(&mut self, jsx: JSXElement) -> Vec<ModuleDecl> {
        self.visit_jsx_element(&jsx);

        let arr = ArrayLit {
            span: DUMMY_SP,
            elems: self.cur_children.drain(..).collect(),
        };

        let return_stmt = Stmt::Return(ReturnStmt {
            span: DUMMY_SP,
            arg: Some(Box::new(Expr::Array(arr))),
        });

        let render_func = Decl::Fn(FnDecl {
            ident: Ident {
                span: DUMMY_SP,
                sym: JsWord::from("render"),
                optional: false,
            },
            declare: false,
            function: Function {
                params: vec![],
                decorators: vec![],
                span: DUMMY_SP,
                is_generator: false,
                is_async: false,
                type_params: None,
                return_type: None,
                body: Some(BlockStmt {
                    span: DUMMY_SP,
                    stmts: vec![return_stmt],
                }),
            },
        });

        vec![ModuleDecl::ExportDecl(ExportDecl {
            span: DUMMY_SP,
            decl: render_func,
        })]
    }
}

impl Visit for JSXTransform {
    fn visit_jsx_element(&mut self, jsx_el: &JSXElement) {
        match &jsx_el.opening.name {
            JSXElementName::Ident(id) => {
                if id
                    .sym
                    .chars()
                    .nth(0)
                    .map_or(false, |first_char| first_char.is_lowercase())
                {
                    jsx_el.visit_children_with(self);

                    let children: Vec<Option<ExprOrSpread>> = self.cur_children.drain(..).collect();

                    self.cur_children.push(html_open_tag!(&*id.sym));
                    self.cur_children.push(array_lit!(children));
                    self.cur_children.push(html_close_tag!(&*id.sym));
                }
            }
            _ => (),
        }
    }

    fn visit_jsx_expr_container(&mut self, expr_cont: &swc_ecma_ast::JSXExprContainer) {
        if let JSXExpr::Expr(e) = &expr_cont.expr {
            self.cur_children.push(Some(ExprOrSpread {
                spread: None,
                expr: e.to_owned(),
            }));
        }
    }

    fn visit_jsx_text(&mut self, txt: &swc_ecma_ast::JSXText) {
        self.cur_children.push(str_lit!(
            JsWord::from(format!("`{}`", &*txt.value)),
            txt.value.clone()
        ));
    }
}

struct ServerTransform;

impl ServerTransform {
    pub fn transform(server_block: ServerBlock) -> Vec<Stmt> {
        server_block.block.stmts
    }
}
