/**
 * We have to identify the reac
 */

function log(arg) {
  let t = 1;
  $: console.log(arg, t);
}

let value = 0;

const doubleValue = value * 2;

// log the value now and whenever it changes
log(doubleValue);

value = 10; // set a new value
