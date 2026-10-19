export interface Box<T> {
  r value: T;
}

export class Counter {
  private rw total: number = 0;
  get count(): number {
    return this.total;
  }
  add(this: Counter, n: number): void {
    this.total += n;
  }
}

export function make<T>(v: T): type Box<T> {
  provide Box<T>
  return { value: v };
}
