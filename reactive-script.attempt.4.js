import logError from "log";

export function log(arg) {
  $: console.log(arg);
}

let value = 10;

log(value);
logError(value);

value = 20;
