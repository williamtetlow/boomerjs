use anyhow::{anyhow, Result};
use id_arena::Arena;
use swc_atoms::JsWord;
use swc_ecma_ast::{Expr, Module, Pat};
use swc_ecma_visit::{Visit, VisitWith};

type ScopeId = id_arena::Id<Scope>;

#[derive(Debug, Default)]
struct ReactiveStatement {
    signals: Vec<JsWord>,
}

#[derive(Default, Debug)]
struct Scope {
    id: Option<ScopeId>,
    children: Vec<ScopeId>,
    parent: Option<ScopeId>,

    params: Vec<JsWord>,
    var_decls: Vec<JsWord>,
    reactive_statements: Vec<ReactiveStatement>,
}

#[derive(Debug, Default)]
struct ReactiveGraph {
    arena: Arena<Scope>,
    root: Scope,
}

#[derive(Default)]
struct Context {
    arena: Option<Arena<Scope>>,
    scope_stack: Vec<Scope>,
    cur_reactive_stmt: Option<ReactiveStatement>,
}

struct Parser<'a> {
    ast: &'a Module,
    context: Context,
}

impl<'a> Parser<'a> {
    fn new(ast: &'a Module) -> Self {
        Parser {
            ast,
            context: Context::default(),
        }
    }

    fn parse_module(&mut self) -> Result<ReactiveGraph> {
        let arena = Arena::<Scope>::new();
        self.context.arena = Some(arena);

        let scope = Scope::default();
        self.context.scope_stack.push(scope);

        self.ast.visit_with(self);

        match self.context.scope_stack.len() {
            1 => {
                let scope = self
                    .context
                    .scope_stack
                    .pop()
                    .expect("scope is in the stack");

                Ok(ReactiveGraph {
                    arena: self.context.arena.take().unwrap(),
                    root: scope,
                })
            }
            0 => Err(anyhow!("unexpected: scope stack is empty")),
            _ => Err(anyhow!("unexpected: scope stack has more than one scope")),
        }
    }

    fn alloc_scope(&mut self, scope: Scope) -> ScopeId {
        if let Some(arena) = self.context.arena.as_mut() {
            arena.alloc(scope)
        } else {
            panic!("arena is None")
        }
    }

    fn register_scope(&mut self, scope: ScopeId) {
        self.update_last_scope(|parent| parent.children.push(scope));
    }

    fn register_reactive_statment(&mut self, r_stmt: ReactiveStatement) {
        self.update_last_scope(|parent| parent.reactive_statements.push(r_stmt));
    }

    fn register_var_decl(&mut self, v_decl: JsWord) {
        self.update_last_scope(|parent| parent.var_decls.push(v_decl));
    }

    fn update_last_scope<F>(&mut self, update_fn: F)
    where
        F: FnOnce(&mut Scope) -> (),
    {
        if let Some(parent) = self.context.scope_stack.last_mut() {
            update_fn(parent);
        } else {
            panic!("no parent")
        }
    }

    fn get_or_set_last_scope_id(&mut self) -> ScopeId {
        if let Some(parent) = self.context.scope_stack.last_mut() {
            if let Some(id) = parent.id {
                id
            } else {
                let id = self.context.arena.as_ref().unwrap().next_id();
                parent.id = Some(id);
                id
            }
        } else {
            panic!("no parent")
        }
    }
}

impl<'a> Visit for Parser<'a> {
    fn visit_fn_decl(&mut self, n: &swc_ecma_ast::FnDecl) {
        let mut scope = Scope::default();

        for param in &n.function.params {
            if let Pat::Ident(id) = &param.pat {
                scope.params.push(id.id.sym.clone());
            }
        }

        self.context.scope_stack.push(scope);

        n.visit_children_with(self);

        let mut scope = self
            .context
            .scope_stack
            .pop()
            .expect("scope is in the stack");

        let has_reactive_stamements = !scope.reactive_statements.is_empty();
        let has_children = !scope.children.is_empty();
        let has_been_assigned_id = scope.id.is_some();

        if has_reactive_stamements || has_children || has_been_assigned_id {
            let parent_id = self.get_or_set_last_scope_id();
            scope.parent = Some(parent_id);
            let scope = self.alloc_scope(scope);
            self.register_scope(scope);
        }
    }

    fn visit_labeled_stmt(&mut self, n: &swc_ecma_ast::LabeledStmt) {
        match &*n.label.sym {
            "$" => {
                if self.context.cur_reactive_stmt.is_some() {
                    panic!("nested reactive statements is not supported")
                }

                match &*n.body {
                    swc_ecma_ast::Stmt::Expr(e) => {
                        self.context.cur_reactive_stmt = Some(ReactiveStatement::default());
                        e.visit_with(self);
                        let r_stmt = self
                            .context
                            .cur_reactive_stmt
                            .take()
                            .expect("reactive statement exists");
                        self.register_reactive_statment(r_stmt);
                    }
                    _ => panic!("only expression statements are supported in reactive statement"),
                }
            }
            _ => (),
        }
    }

    fn visit_var_declarator(&mut self, n: &swc_ecma_ast::VarDeclarator) {
        match &n.name {
            Pat::Ident(id) => self.register_var_decl(id.id.sym.clone()),
            _ => (),
        }
    }

    fn visit_call_expr(&mut self, n: &swc_ecma_ast::CallExpr) {
        if let Some(cur_reactive_stmt) = self.context.cur_reactive_stmt.as_mut() {
            for arg in &n.args {
                match &*arg.expr {
                    Expr::Ident(id) => cur_reactive_stmt.signals.push(id.sym.clone()),
                    _ => (),
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use swc_common::{
        errors::{ColorConfig, Handler},
        sync::Lrc,
        SourceMap,
    };
    use swc_ecma_parser::{
        lexer::Lexer, Capturing, Parser as SWCParser, StringInput, Syntax, TsConfig,
    };

    use super::Parser;

    #[test]
    fn it_does_something() {
        let source_map: Lrc<SourceMap> = Default::default();
        let handler =
            Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(source_map.clone()));

        let source_file = source_map.new_source_file(
            swc_common::FileName::Internal("test.js".into()),
            "
function log(x) {
    $: console.log(x)

    function someInternalFuncion() {
      return true
    }
}

function error(x) {
    $: console.error(x)
}

let value = 10

log(value)

value = 20"
                .into(),
        );

        let lexer = Lexer::new(
            Syntax::Typescript(TsConfig {
                tsx: true,
                ..Default::default()
            }),
            Default::default(),
            StringInput::from(&*source_file),
            None,
        );

        let mut swc_parser = SWCParser::new_from(Capturing::new(lexer));

        for e in swc_parser.take_errors() {
            e.into_diagnostic(&handler).emit();
        }

        let module = swc_parser
            .parse_module()
            .map_err(|e| e.into_diagnostic(&handler).emit())
            .expect("failed to parse");

        let mut parser = Parser::new(&module);

        let result = parser.parse_module().expect("failed to parse r_graph");

        println!("result: {:#?}", &result);
    }
}
