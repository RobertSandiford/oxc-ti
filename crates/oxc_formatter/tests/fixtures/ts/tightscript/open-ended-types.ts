type A1<T> = (distribute T as RealKeyOf<T> & string | `${RealKeyOf<T> & number}`)[];
type A2<T> = (distribute T as T) | X;
type A3<T> = X | distribute T as T;
type A4<T> = rw (distribute T as T);
type A5<T> = owned readwrite (distribute T as T[])[];
declare function f(): (type T) | X;
