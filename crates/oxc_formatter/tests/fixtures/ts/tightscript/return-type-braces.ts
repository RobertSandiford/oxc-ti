// At the top of a function's `: T` return type, `rw {}` would read as a type named `rw` and a body.
declare function f1(): (rw {});
declare function f2(): (deep rw {});
declare function f3(): Box<rw {}>;
declare function f4(): rw { a: string };
declare function f5(): (transient {});
declare function f6(): () => (r {});
declare function f7(): (owned {});
declare function f8(): (owned rw {});
const a1 = (): (rw {}) => ({});
interface I {
  m(): (rw {});
  n(): (rw {}) | null;
  o(): keyof (rw {});
  q(): (rw {})[];
}
class C {
  m(): (rw {}) | null {
    return null;
  }
  get g(): keyof (rw {}) {
    return "";
  }
}
declare function c1(): (rw {}) extends A ? B : C;
declare function p1(x: unknown): x is (rw {});
declare function r1(): R and type (rw {});
declare function r2(): type (rw {});
type F = () => rw {};
let v: rw {} = {};
