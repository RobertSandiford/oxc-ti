import { describe, expect, it } from "vitest";

import { blankTypesSync, compileSync } from "../index";

describe("TightScript blankTypesSync", () => {
  it("keeps every line and column", () => {
    const { code, errors } = blankTypesSync("a.ts", "let x: rw { a: string } = { a: '' };");
    expect(errors).toEqual([]);
    expect(code).toBe("let x                   = { a: '' };");
  });
});

describe("TightScript compileSync", () => {
  it("removes types and compiles JSX to the automatic runtime", () => {
    const source = "export const App = (p: owned { n: number }) => <b>{p.n}</b>;\n";
    const { code, map, errors } = compileSync("app.tsx", source, {
      jsxImportSource: "tungsten",
      sourceMap: true,
    });
    expect(errors).toEqual([]);
    expect(code).toContain('from "tungsten/jsx-runtime"');
    expect(code).not.toContain("owned");
    expect(JSON.parse(map!).sources).toEqual(["app.tsx"]);
  });

  it("leaves the source map out unless asked", () => {
    const { map } = compileSync("a.ts", "export const a: number = 1;");
    expect(map).toBeUndefined();
  });

  it("runs tungsten's passes and returns their output", () => {
    const source = [
      "import { A } from './A.tsx';",
      "import 'pkg/b.css';",
      "import 'pkg/side';",
      "export const s = Scss`.x { color: red }`;",
      "export const e = <A />;",
    ].join("\n");
    const { code, errors, tungsten } = compileSync("app.tsx", source, {
      jsxImportSource: "tungsten",
      tungsten: JSON.stringify({
        jsxExtensionsToJs: true,
        packageImports: { replacements: { "pkg/b.css": "/.packages/pkg@1.0.0/b.css" } },
        componentStyles: {},
      }),
    });
    expect(errors).toEqual([]);
    expect(code).toContain('"./A.js"');
    expect(code).toContain('"/.packages/pkg@1.0.0/b.css"');
    const output = JSON.parse(tungsten!);
    expect(output.bareImports).toEqual(expect.arrayContaining(["pkg/b.css", "pkg/side"]));
    expect(output.componentStyles).toEqual([
      { index: 1, tag: "Scss", raw: ".x { color: red }", inClass: false },
    ]);
  });

  it("reports invalid tungsten options as an error", () => {
    const { errors } = compileSync("a.ts", "1;", { tungsten: '{"nope":1}' });
    expect(errors.length).toBe(1);
  });

  it("reports parse errors", () => {
    const { errors } = compileSync("a.ts", "let x: = 1;");
    expect(errors.length).toBe(1);
  });
});
