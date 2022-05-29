let reactiveContexts = new WeakMap();

let scope = {
  _value: 0,
  get value() {
    return scope._value;
  },
  set value(a) {
    scope._value = a;
    scope.$1.call(reactiveContexts.get(scope.$1), scope.doubleValue, scope.t);
  },
  get doubleValue() {
    return scope.value * 2;
  },
  t: 1,
  $1: function (x) {
    reactiveContexts.set(scope.$1, this);
    console.log(x, 0);
  },
};

function log(arg) {
  scope.$1.call(this, arg);
}

log(scope.doubleValue);

scope.value = 10;
