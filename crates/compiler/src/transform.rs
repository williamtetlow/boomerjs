use std::ops::{Deref, DerefMut};

use swc_atoms::JsWord;
use swc_common::{Spanned, DUMMY_SP};
use swc_ecma_ast::{
    ArrayLit, BlockStmt, CallExpr, Decl, ExportDecl, Expr, ExprOrSpread, FnDecl, Function, Ident,
    JSXAttr, JSXElement, JSXElementChild, JSXElementName, JSXExpr, JSXExprContainer, JSXText, Lit,
    Module, ModuleDecl, ModuleItem, ReturnStmt, Stmt, Str,
};
use swc_ecma_visit::{Visit, VisitWith};

use crate::parser::{ClientBlock, ParseResult, ServerBlock};

pub struct BmrTransform;

impl BmrTransform {
    pub fn transform(parse_result: ParseResult) -> Module {
        let mut jsx_transform = JSXTransform::new(&parse_result.client);
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

#[derive(Default, Clone, Copy)]
pub struct JSXTransformContext {
    // <button onClick={...
    in_attribute_expr: bool,
    // <div>{...
    in_child_expr: bool,
}

pub struct JSXTransform<'a> {
    cur_children: Vec<Option<ExprOrSpread>>,

    ctx: JSXTransformContext,

    client_block: &'a Option<ClientBlock>,

    jsx_el_stack: Vec<Vec<Option<ExprOrSpread>>>,
}

impl<'a> JSXTransform<'a> {
    pub fn new(client_block: &'a Option<ClientBlock>) -> Self {
        Self {
            cur_children: Default::default(),
            ctx: Default::default(),
            client_block,
            jsx_el_stack: Default::default(),
        }
    }
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

    fn is_get_state(&self, id: JsWord) -> bool {
        self.client_block
            .as_ref()
            .map_or(false, |block| block.use_state.get.contains(&id))
    }

    fn is_set_state(&self, id: JsWord) -> bool {
        self.client_block
            .as_ref()
            .map_or(false, |block| block.use_state.set.contains(&id))
    }

    fn with_ctx(&'a mut self, ctx: JSXTransformContext) -> WithContext<'a> {
        let orig_ctx = self.ctx;
        self.set_ctx(ctx);

        WithContext {
            inner: self,
            orig_ctx,
        }
    }

    fn set_ctx(&mut self, ctx: JSXTransformContext) {
        self.ctx = ctx;
    }
}

pub struct WithContext<'a> {
    inner: &'a mut JSXTransform<'a>,
    orig_ctx: JSXTransformContext,
}

impl<'a> Deref for WithContext<'a> {
    type Target = JSXTransform<'a>;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl<'a> DerefMut for WithContext<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner
    }
}

impl<'a> Drop for WithContext<'a> {
    fn drop(&mut self) {
        self.inner.set_ctx(self.orig_ctx);
    }
}

impl<'a> Visit for JSXTransform<'a> {
    fn visit_jsx_element(&mut self, jsx_el: &JSXElement) {
        match &jsx_el.opening.name {
            JSXElementName::Ident(id) => {
                if id
                    .sym
                    .chars()
                    .nth(0)
                    .map_or(false, |first_char| first_char.is_lowercase())
                {
                    self.cur_children = vec![];

                    self.jsx_el_stack.push(vec![]);

                    jsx_el.visit_children_with(self);

                    let children: Vec<Option<ExprOrSpread>> =
                        self.jsx_el_stack.pop().unwrap_or(vec![]);

                    self.cur_children.push(html_open_tag!(&*id.sym));
                    self.cur_children.push(array_lit!(children));
                    self.cur_children.push(html_close_tag!(&*id.sym));
                }
            }
            _ => (),
        }
    }

    fn visit_jsx_attr(&mut self, attr: &JSXAttr) {
        self.ctx.in_attribute_expr = true;
        // TODO: if is client side attribute (e.g onClick)
        attr.visit_children_with(self);

        self.ctx.in_attribute_expr = false;
    }

    fn visit_jsx_element_children(&mut self, children: &[JSXElementChild]) {
        // here we have to ch

        for child in children {
            /*
             * We are going to have to do two passes of the JSX
             *
             * 1. To identify the client side holes
             * 2. To transform
             *
             * During 1. we should:
             *
             * 1. Record the spans of holes that need clientside reactivity.
             * 2. Record the reactive dependants for that span
             *
             * During 2. when we come across a clientside hole we will need to track a bit more
             *
             * We need to:
             * 1. Pull out a template that can be used for updating on the client with innerHTML
             * 2. Attach selector to the element so we can find it in the DOM
             * 3. Generate reactive get and set
             */
            let _span = child.span();

            child.visit_with(self);
        }
    }

    fn visit_jsx_element_child(&mut self, child: &JSXElementChild) {
        match child {
            JSXElementChild::JSXText(t) => t.visit_with(self),
            JSXElementChild::JSXExprContainer(e) => {
                self.ctx.in_child_expr = true;
                e.visit_with(self);
                self.ctx.in_child_expr = false;
            }
            JSXElementChild::JSXSpreadChild(s) => s.visit_with(self),
            JSXElementChild::JSXElement(e) => e.visit_with(self),
            JSXElementChild::JSXFragment(f) => f.visit_with(self),
        }
    }

    fn visit_jsx_expr_container(&mut self, expr_cont: &JSXExprContainer) {
        if let JSXExpr::Expr(e) = &expr_cont.expr {
            if let Some(last) = self.jsx_el_stack.last_mut() {
                last.push(Some(ExprOrSpread {
                    spread: None,
                    expr: e.to_owned(),
                }));
            }
        }
    }

    fn visit_call_expr(&mut self, call: &CallExpr) {}

    fn visit_jsx_text(&mut self, txt: &JSXText) {
        if let Some(last) = self.jsx_el_stack.last_mut() {
            last.push(str_lit!(
                JsWord::from(format!("`{}`", &*txt.value)),
                txt.value.clone()
            ));
        }
    }
}

struct ServerTransform;

impl ServerTransform {
    pub fn transform(server_block: ServerBlock) -> Vec<Stmt> {
        server_block.block.stmts
    }
}

// struct ClientTransform;

// impl ClientTransform {
//     pub fn transform(&mut self, client_block: ClientBlock) ->
// }

// impl Visit for ClientTransform {

// }
