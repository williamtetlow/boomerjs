let ctxMap = new WeakMap();

let hoist = {
  $: function (arg) {
    ctxMap.set(hoist.$, this);
    console.error(arg);
  },
};

function call(hoisted, ...params) {
  let ctx = ctxMap.get(hoisted);
  hoisted.call(ctx, ...params);
}

export default function logError(arg) {
  hoist.$.call(this, arg);
}

export function setLogErrorParams(...params) {
  call(hoist.$, ...params);
}
