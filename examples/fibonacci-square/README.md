This is the fibonacci vadcop example.
To use it from scratch you need to install pil2-compiler and pil2-components and pil2-proofman-js (feature/setup branch)

To compile the PIL files:

```
node ../pil2-compiler/src/pil.js ../pil2-components/test/fibonacci/pil/build.pil -I ../pil2-components/lib/std/pil -o ../pil2-components/test/fibonacci/pil/build.pilout
```

To generate the setup files:

```
node ../pil2-proofman-js/src/setup/main_genSetup.js -a ../pil2-components/test/fibonacci/pil/build.pilout -b ./examples/fibonacci-vadcop/build
```
