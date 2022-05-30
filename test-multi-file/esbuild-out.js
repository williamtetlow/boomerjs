(() => {
  // test-multi-file/output/log.js
  var ctxMap = /* @__PURE__ */ new WeakMap();
  var hoist = {
    $: function(arg) {
      ctxMap.set(hoist.$, this);
      console.log(arg);
    }
  };
  function call(hoisted, ...params) {
    let ctx = ctxMap.get(hoisted);
    hoisted.call(ctx, ...params);
  }
  function log(arg) {
    hoist.$.call(this, arg);
  }
  function setLogParams(...params) {
    call(hoist.$, ...params);
  }

  // test-multi-file/output/logError.js
  var ctxMap2 = /* @__PURE__ */ new WeakMap();
  var hoist2 = {
    $: function(arg) {
      ctxMap2.set(hoist2.$, this);
      console.error(arg);
    }
  };
  function call2(hoisted, ...params) {
    let ctx = ctxMap2.get(hoisted);
    hoisted.call(ctx, ...params);
  }
  function logError(arg) {
    hoist2.$.call(this, arg);
  }
  function setLogErrorParams(...params) {
    call2(hoist2.$, ...params);
  }

  // test-multi-file/output/index.js
  var value = 10;
  function setValue(v) {
    value = v;
    setLogParams(value);
    setLogErrorParams(value);
  }
  log(value);
  logError(value);
  setValue(100);
})();
