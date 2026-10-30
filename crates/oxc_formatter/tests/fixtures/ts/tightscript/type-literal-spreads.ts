type S1 = { ...A, ...B };
type S2 = { ...A; ...B };
type S3 = {
    ...A
    ...B
};
type S4 = { ...A, b: number, ...B, c?: boolean };
type S5 = { ...A | undefined, ...(B | null) };
type S7 = { ...{ ...A, ...B } };
declare function f<T extends { ...A }>(x: { ...T, ...B }): { ...T };
