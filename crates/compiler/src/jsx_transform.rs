use swc_atoms::JsWord;
use swc_common::DUMMY_SP;
use swc_ecma_ast::{
    ArrayLit, Expr, ExprOrSpread, JSXElement, JSXElementName, JSXExpr, Lit, ModuleDecl, ReturnStmt,
    Stmt, Str,
};
use swc_ecma_visit::{Visit, VisitWith};

#[derive(Default)]
struct JSXTransform {
    module_decls: Vec<ModuleDecl>,
    stmts: Vec<Stmt>,

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
    pub fn transform_jsx(&mut self, jsx: &JSXElement) {
        self.visit_jsx_element(jsx);

        let arr = ArrayLit {
            span: DUMMY_SP,
            elems: self.cur_children.drain(..).collect(),
        };

        let stmt = Stmt::Return(ReturnStmt {
            span: DUMMY_SP,
            arg: Some(Box::new(Expr::Array(arr))),
        });

        self.stmts.push(stmt);
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
            JsWord::from(format!(r#""{}""#, &*txt.value)),
            txt.value.clone()
        ));
    }
}

#[cfg(test)]
mod test {
    use swc_common::{
        errors::{ColorConfig, Handler},
        sync::Lrc,
        FileName, SourceMap, DUMMY_SP,
    };
    use swc_ecma_ast::{Module, ModuleItem};
    use swc_ecma_codegen::{text_writer::JsWriter, Emitter};

    use crate::parser::Parser;

    use super::JSXTransform;

    #[test]
    fn test() {
        let input = "<div><h1>{hello}</h1><h2>{world}</h2></div>";
        let source_map: Lrc<SourceMap> = Default::default();
        let handler =
            Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(source_map.clone()));

        let mut parser = Parser::new(source_map.clone(), &handler);

        let source_file =
            source_map.new_source_file(FileName::Custom("./test.js".into()), input.to_owned());

        let result = parser.parse(source_file).expect("failed to parse");

        let jsx = result.jsx.unwrap();

        let mut transformer = JSXTransform::default();

        transformer.transform_jsx(&jsx);

        println!("{:#?}", &transformer.stmts);

        let mut buf = vec![];

        let mut swc_emitter = Emitter {
            cfg: swc_ecma_codegen::Config { minify: false },
            cm: source_map.clone(),
            comments: None,

            wr: JsWriter::new(source_map.clone(), "\n", &mut buf, None),
        };

        let module = Module {
            span: DUMMY_SP,
            shebang: None,
            body: transformer
                .stmts
                .into_iter()
                .map(|stmt| ModuleItem::Stmt(stmt))
                .collect(),
        };

        swc_emitter.emit_module(&module).unwrap();

        println!("{:?}", String::from_utf8_lossy(&buf));
    }
}
