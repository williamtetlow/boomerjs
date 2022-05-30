import { default as log, setLogParams } from "./log";
import { default as logError, setLogErrorParams } from "./logError";

let value = 10;

function setValue(v) {
  value = v;
  setLogParams(value);
  setLogErrorParams(value);
}

log(value);
logError(value);

setValue(100);
