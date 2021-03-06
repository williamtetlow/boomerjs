import { logError, setLogErrorParams } from "log";

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

export function log(arg) {
  hoist.$.call(this, arg);
}

export function setLogParams(...params) {
  call(hoist.$, ...params);
}

let value = 0;

function setValue(v) {
  value = v;
  call(hoist.$, value);
  setLogErrorParams(value);
}

log(value);
logError(value);

// value = 10
setValue(10);
