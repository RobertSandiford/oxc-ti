type M1 = { read   a: string; rw b: number; readonly c: boolean };
interface M2 { write a: string; r b: number }
type M3 = [read a: string, delete b?: number, rw ...rest: string[]];
class M4 {
    read x = 1;
    private rw y: string = "y";
    static r z = 3;
    declare w q: number;
    accessor rw t = 1;
    static rw [k: string]: unknown;
}
type I = { read [key: string]: T };

type MP1<T> = { read [K in keyof T]: T[K] };
type MP2<T> = { -write [K in keyof T]: T[K] };
type MP3<T> = { readonly +rw [K in keyof T]?: T[K] };

type Names = { read: string; rw(): void; r?: number };
class Names2 { rw = 3; r() {} }
