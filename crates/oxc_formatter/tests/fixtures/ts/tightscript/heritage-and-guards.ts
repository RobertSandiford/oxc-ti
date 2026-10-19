interface Child extends -exact Base { b: string }
interface Two extends A,   -exact B<T> {}
class D implements -exact A { a = 1 }

function isStr(x: unknown): x is?   string { return typeof x === "string"; }
class G { isMe(): this is? G { return true; } }
function assertStr(x: unknown): asserts x is? string {}
