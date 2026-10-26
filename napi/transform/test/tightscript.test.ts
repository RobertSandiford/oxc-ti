import { describe, expect, it } from "vitest";

import { blankTypesSync } from "../index";

describe("TightScript blankTypesSync", () => {
  it("keeps every line and column", () => {
    const { code, errors } = blankTypesSync("a.ts", "let x: rw { a: string } = { a: '' };");
    expect(errors).toEqual([]);
    expect(code).toBe("let x                   = { a: '' };");
  });
});
