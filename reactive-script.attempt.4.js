import logError from "log";

function log(arg) {
  $: console.log(arg);
}

let value = 10;

log(value);
logError(value);

value = 20;
