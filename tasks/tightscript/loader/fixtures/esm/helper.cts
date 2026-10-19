function twice(n: number): number {
  return n * 2;
}

module.exports = { twice } satisfies Record<string, (n: number) => number>;
