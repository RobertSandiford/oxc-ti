import { Counter, make, type Box } from "./lib.js";
import { twice } from "./helper.cjs";

const counter: owned rw Counter = new Counter();
counter.add(2);

export type Made from make(3);

const box: r Box<number> = { value: twice(counter.count) as number };

function fail(): never {
  throw new Error("thrown at line 12");
}

console.log(JSON.stringify({ count: counter.count, value: box.value }));
try {
  fail();
} catch (error) {
  console.log((error as Error).stack!.split("\n")[1].trim());
}
