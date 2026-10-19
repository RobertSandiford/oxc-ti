type D1<U> = distribute   U as U[];
type D2<U> = distribute U M as { m: M };
type D3<U> = string | distribute U as U[];

type F = (a: string) => string;
export function f1: F (a) { return a }
export function f3: (GF) <T>(a: T) { return a }
export async function f4: ((a: string) => Promise<string>) (a) { return a }
export declare function f6: F;

export function f<G>(v: G): type Box<G> {}
export function hy<G>(v: G): boolean and type Box<G> { return true; }
export type TF = <G>(v: G) => type Box<G>;

function p1(): number {
    provide Box<string>
    return 1;
}
function p2<T>(v: T) {
    provide from
    g(v);
}
function p3<T>(v: T) {
    provide type B = Box<T>
    const b: B = { value: v };
    return b;
}
function p5<T>(v: T) {
    provide type G from
    const n = gv(v);
    return n;
}

export type R from f(1);
export type RN from
f<string>('s');
export type RV from
export const rv = hy(1);
