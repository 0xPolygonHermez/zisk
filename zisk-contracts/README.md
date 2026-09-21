# zisk-contracts

Solidity verifier contracts for the Zisk zkVM, plus a Hardhat test that submits
a wrapped PLONK proof on-chain.

## Running the verifier test

All commands below are run from this directory:

```bash
cd zisk-contracts
```

Export the four ABI fields the on-chain verifier expects. The subcommand is
developer-facing, so it lives in `cargo-zisk-dev` rather than `cargo zisk`:

```bash
cargo build --release -p cargo-zisk --bin cargo-zisk-dev
../target/release/cargo-zisk-dev export-solidity-calldata -p test/plonk_proof.bin -o fixtures/fixture.json
```

Install Hardhat deps (first run only) and run the test:

```bash
npm install
ZISK_SOLIDITY_FIXTURE=fixtures/fixture.json npx hardhat test
```