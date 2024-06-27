
# PIL2 Proofman

PIL2 Proofman is a versatile framework designed to help the development, orchestration and validation of zero-knowledge proofs. Powered by PIL2, a robust and easy-to-audit language adept at expressing polynomial identities, it is essential for describing from basic statements to the integration of multiple coprocessors within custom zkVMs. The PIL2 toolkit provides a the ['PIL2 compiler'](https://github.com/0xPolygonHermez/pilcom), a library of reusable ['PIL2 components'](https://github.com/0xPolygonHermez/pil2-components), and other debugging tools to ensure the correctness of your PIL2 project. Upon compiling a PIL2 project a binary file named PILOUT is generated. The PILOUT files encapsulate arithmetization, public inputsm constraints, constant values and other proof-generation-specific details described by a PIL2 project.

PIL2 Proofman is the framework to be used when generate the zero-knowledge proofs. PIL2 Proofman requires a PILOUT file, some Rust code snippets to fille the witnesses (for all the subprofs) and a prover. While PIL2 Proofman includes a library of provers, users can also develop and integrate their own. The current provided provers includes STARK and FFLONK.

|[PIL2 toolkit](./pil2/...)|[PIL2 Proofman](./pil2/...)|[Quickstart](./pil2/...)|[Examples](./pil2/...)|[Docs](./pil2/...)|

**PIL2 Proofman is in alpha version. Do not use it in production environments**

## PIL2 Proofman system requirements

Before start, make sure you have [Rust](https://www.rust-lang.org/tools/install) installed in your system.

Optional recommendations:

- [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer) extension if you are using VS Code to assist you when writing Rust code.
- [PIL2 Highlight syntax code](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer) if you are using VS Code to highlight your code when writing PIL2 code.

## License

PIL2 Proofman has a XXX license. Please check the [COPYING](TODO) file for more details.
<!--
Provided provers:

|Prover|Status|Type|Description |Since|
|---------|---|----|------------|-----|
|STARK|Available|STARK|Performant Stark prover used by Polygon zkEVM|v0.1.0|
|FFLONK|Work in Progress|SNARK|Performant Fflonk prover used by Polygon zkEVM|v0.1.0|

**PIL2 Proofman is in alpha version. Do not use it in production environments**

## Proofman toolchain

To assist you when starting or maintaining a project we've designed a toolchain that will provide you some commands to simplify some tasks.

## Proofman examples

We have some example to provide you from simple use cases to a more complex ones that can be found in the [examples folder](https://github.com/0xPolygonHermez/pil2-proofman/tree/main/examples)

There is also another more comple example that generates the proof for the Polygon zkEVM that can be found [here](https://github.com/0xPolygonHermez/zkevm-prover-rust)
 -->
