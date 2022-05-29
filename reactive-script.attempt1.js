/* let value = 10;

$: console.log(value);

value = 20; */

const ast = {
  body: [
    { type: "declaration", left: "value", right: 10 },
    {
      type: "labeled_stmt",
      body: { type: "func_call", func: "console.log", params: ["value"] },
    },
    {
      type: "expression",
      body: { type: "assignment", left: "value", right: 20 },
    },
  ],
};

const default_V = {
  declaration: (node, state) => {},
  labeled_stmt: (node, state) => {},
  func_call: (node, state) => {},
  expression: (node, state) => {},
  assignment: (node, state) => {},
};

const reactiveStatements = [];

const findReactiveStmts_V = {
  ...default_V,
  labeled_stmt: (node, state) => {
    switch (node.body.type) {
      case "func_call": {
        let reactiveStatement = {
          dependants: node.body.params,
        };
        reactiveStatements.push(reactiveStatement);
        break;
      }
      default:
      // do nothing
    }
  },
};

function visitWith(ast, visitor) {
  for (const item of ast.body) {
    visitor[item.type](item);
  }
}

// 1. find the reactive statements
visitWith(ast, findReactiveStmts_V);

console.log("reactiveStatements", reactiveStatements);

// 2. build reactive graph
/**
 * definition -> statement <- assignment
 */
const reactiveGraph = {};
const buildReactiveGraph_V = {
  ...default_V,
  declaration: (node, state) => {
    const stmt = reactiveStatements.find((x) =>
      x.dependants.contains(node.right)
    );

    if (!stmt) return;
  },
};
