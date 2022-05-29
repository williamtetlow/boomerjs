use anyhow::{anyhow, Result};
use id_arena::Arena;
use swc_atoms::JsWord;
use swc_ecma_ast::{Module, Pat};
use swc_ecma_visit::{Visit, VisitWith};

type ScopeId = id_arena::Id<Scope>;

#[derive(Debug)]
struct ReactiveStatement {}

#[derive(Default, Debug)]
struct Scope {
    params: Vec<JsWord>,
    var_decls: Vec<JsWord>,
    reactive_statements: Vec<ReactiveStatement>,
    scopes: Vec<ScopeId>,
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

        let scope = self
            .context
            .scope_stack
            .pop()
            .expect("scope is in the stack");

        let parent_scope = self
            .context
            .scope_stack
            .last_mut()
            .expect("parent scope is in the stack");

        let scope = self.context.arena.as_mut().unwrap().alloc(scope);
        parent_scope.scopes.push(scope);
    }

    fn visit_labeled_stmt(&mut self, n: &swc_ecma_ast::LabeledStmt) {
        match &*n.label.sym {
            "$" => {
                let scope = self
                    .context
                    .scope_stack
                    .last_mut()
                    .expect("stack not empty");

                scope.reactive_statements.push(ReactiveStatement {});
            }
            _ => (),
        }
    }

    fn visit_var_declarator(&mut self, n: &swc_ecma_ast::VarDeclarator) {
        let scope = self
            .context
            .scope_stack
            .last_mut()
            .expect("stack not empty");

        match &n.name {
            Pat::Ident(id) => scope.var_decls.push(id.id.sym.clone()),
            _ => (),
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
