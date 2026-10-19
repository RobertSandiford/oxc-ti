# Exit code
1

# stdout
```
  ! eslint(no-unused-vars): Type alias 'D' is declared but never used.
   ,-[files/index.ts:1:6]
 1 | type D<U> = distribute U as Box<U>;
   :      |
   :      `-- 'D' is declared here
 2 | 
   `----
  help: Consider removing this declaration.

  x tightscript-visit(report-nodes): Visited TSDistributeType
   ,-[files/index.ts:1:13]
 1 | type D<U> = distribute U as Box<U>;
   :             ^^^^^^^^^^^^^^^^^^^^^^
 2 | 
   `----

  x tightscript-visit(report-nodes): Reference U in TSDistributeType
   ,-[files/index.ts:1:24]
 1 | type D<U> = distribute U as Box<U>;
   :                        ^
 2 | 
   `----

  x tightscript-visit(report-nodes): Reference Box in TSDistributeType
   ,-[files/index.ts:1:29]
 1 | type D<U> = distribute U as Box<U>;
   :                             ^^^^^^
 2 | 
   `----

  x tightscript-visit(report-nodes): Reference U in TSTypeParameterInstantiation
   ,-[files/index.ts:1:33]
 1 | type D<U> = distribute U as Box<U>;
   :                                 ^
 2 | 
   `----

  ! eslint(no-unused-vars): Parameter 'v' is declared but never used. Unused parameters should start with a '_'.
   ,-[files/index.ts:3:32]
 2 | 
 3 | export function returnsType<G>(v: G): type Box<G> {}
   :                                |
   :                                `-- 'v' is declared here
 4 | 
   `----
  help: Consider removing this parameter.

  x tightscript-visit(report-nodes): Reference G in TSTypeAnnotation
   ,-[files/index.ts:3:35]
 2 | 
 3 | export function returnsType<G>(v: G): type Box<G> {}
   :                                   ^
 4 | 
   `----

  x tightscript-visit(report-nodes): Visited TSTypeReturn
   ,-[files/index.ts:3:39]
 2 | 
 3 | export function returnsType<G>(v: G): type Box<G> {}
   :                                       ^^^^^^^^^^^
 4 | 
   `----

  x tightscript-visit(report-nodes): Reference Box in TSTypeReturn
   ,-[files/index.ts:3:44]
 2 | 
 3 | export function returnsType<G>(v: G): type Box<G> {}
   :                                            ^^^^^^
 4 | 
   `----

  x tightscript-visit(report-nodes): Reference G in TSTypeParameterInstantiation
   ,-[files/index.ts:3:48]
 2 | 
 3 | export function returnsType<G>(v: G): type Box<G> {}
   :                                                ^
 4 | 
   `----

  x tightscript-visit(report-nodes): Reference T in TSTypeAnnotation
   ,-[files/index.ts:5:32]
 4 | 
 5 | export function provides<T>(v: T): T {
   :                                ^
 6 |   provide Box<T>
   `----

  x tightscript-visit(report-nodes): Reference T in TSTypeAnnotation
   ,-[files/index.ts:5:36]
 4 | 
 5 | export function provides<T>(v: T): T {
   :                                    ^
 6 |   provide Box<T>
   `----

  x tightscript-visit(report-nodes): Visited TSProvideDeclaration
   ,-[files/index.ts:6:3]
 5 | export function provides<T>(v: T): T {
 6 |   provide Box<T>
   :   ^^^^^^^^^^^^^^
 7 |   return v;
   `----

  x tightscript-visit(report-nodes): Reference Box in TSProvideDeclaration
   ,-[files/index.ts:6:11]
 5 | export function provides<T>(v: T): T {
 6 |   provide Box<T>
   :           ^^^^^^
 7 |   return v;
   `----

  x tightscript-visit(report-nodes): Reference T in TSTypeParameterInstantiation
   ,-[files/index.ts:6:15]
 5 | export function provides<T>(v: T): T {
 6 |   provide Box<T>
   :               ^
 7 |   return v;
   `----

  x tightscript-visit(report-nodes): Visited TSTypeFromDeclaration
    ,-[files/index.ts:10:8]
  9 | 
 10 | export type R from make(1);
    :        ^^^^^^^^^^^
    `----

Found 2 warnings and 14 errors.
Finished in Xms on 1 file with 97 rules using X threads.
```

# stderr
```
```
