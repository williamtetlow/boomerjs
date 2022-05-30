const rGraph = {
  arena: [
    {
      id: 0,
      children: [],
      parent: 2,
      params: ["x"],
      var_decls: [],
      reactive_statements: [
        {
          signals: ["x"],
        },
      ],
      func_calls: [],
    },
    {
      id: 1,
      children: [],
      parent: 2,
      params: ["x"],
      var_decls: [],
      reactive_statements: [
        {
          signals: ["x"],
        },
      ],
      func_calls: [],
    },
    {
      id: 2,
      children: [0, 1],
      parent: undefined,
      params: [],
      var_decls: ["value"],
      reactive_statements: [],
      func_calls: [{ scope: 0, params: ["value"] }],
    },
  ],
  root: 2,
};

function findReactiveBranch(rGraph) {
  const root = rGraph.arena[rGraph.root];

  if (!root) return [];

  let result = [];

  let children = [...root.children];

  for (const childId of children) {
    const child = rGraph.arena[childId];

    children.push(...child.children);

    if (!child.reactive_statements.length) {
      continue;
    }

    for (const rstmt of child.reactive_statements) {
      for (const signal of rstmt.signals) {
        let branch = signal;
        if (child.var_decls.includes(signal)) {
          branch += `->scope_${childId}_signal_${signal}`;
        } else if (child.params.includes(signal)) {
          branch += `->scope_${childId}_param(${signal})`;

          let parent = rGraph.arena[child.parent];
          while (parent) {
            if (!parent.func_calls.length) {
              parent = rGraph.arena[parent.parent];
              continue;
            }
            for (const func_call of parent.func_calls) {
              if (func_call.scope === child.id) {
                branch += `->scope_${parent.id}_call_scope_${child.id}(${func_call.params[0]})->scope_${parent.id}_let_${func_call.params[0]}`;
                parent = rGraph.arena[parent.parent];
                break;
              } else {
                parent = rGraph.arena[parent.parent];
              }
            }
          }
        }

        result.push(branch);
      }
    }
  }

  return result;
}

console.log(findReactiveBranch(rGraph));
