let ctxMap = new WeakMap();

let hoist = {
  $: function (arg) {
    ctxMap.set(hoist.$, this);
    console.log(arg);
  },
};

function call(hoisted, ...params) {
  let ctx = ctxMap.get(hoisted);
  hoisted.call(ctx, ...params);
}

export default function log(arg) {
  hoist.$.call(this, arg);
}

export function setLogParams(...params) {
  call(hoist.$, ...params);
}
