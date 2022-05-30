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

function log(arg) {
  hoist.$.call(this, arg);
}

let value = 0;

function setValue(v) {
  value = v;
  call(hoist.$, value);
}

log(value);

// value = 10
setValue(10);
