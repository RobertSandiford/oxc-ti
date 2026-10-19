const a = <deep rw T,>(x: T) => x;
const b = <const deep rw T,>(x: T) => x;
const c = <deep rw T extends U>(x: T) => x;
