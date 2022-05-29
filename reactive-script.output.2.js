let reactiveContexts = new WeakMap();

// export so it can be imported in other files
export const reactiveVars = {
  _value: 0,
  get value() {
    return scope._value;
  },
  set value(a) {
    scope._value = a;
    $1.call(reactiveContexts.get($1), scope.doubleValue, scope.t);
  },
  get doubleValue() {
    return scope.value * 2;
  },
};

function $1(x) {
  reactiveContexts.set(scope.$1, this);
  console.log(x, 0);
  console.log(this.beans);
}

function log(arg) {
  this.beans = "on toast";
  $1.call(this, arg);
}

log(scope.doubleValue);

scope.value = 10;
