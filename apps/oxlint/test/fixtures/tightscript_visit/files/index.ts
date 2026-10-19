type D<U> = distribute U as Box<U>;

export function returnsType<G>(v: G): type Box<G> {}

export function provides<T>(v: T): T {
  provide Box<T>
  return v;
}

export type R from make(1);
