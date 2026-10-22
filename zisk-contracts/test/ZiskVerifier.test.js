const { expect } = require("chai");
const { ethers } = require("hardhat");
const fs = require("fs");
const path = require("path");

describe("ZiskVerifier", function () {
  // ZISK_SOLIDITY_CONTRACT picks the family's pilfflonk verifier the fixture is for.
  const contract = process.env.ZISK_SOLIDITY_CONTRACT || "blake3/ZiskVerifier.sol:ZiskVerifier";

  it(`verifies a final SNARK proof on-chain (${contract})`, async function () {
    const fxPath =
      process.env.ZISK_SOLIDITY_FIXTURE ||
      path.join(__dirname, "..", "fixtures", "blake3.json");

    if (!fs.existsSync(fxPath)) {
      throw new Error(
        `Fixture not found at ${fxPath}. ` +
          `Generate one with: cargo-zisk-dev export-solidity-calldata -p <proof.bin> -o ${fxPath}`
      );
    }

    const f = JSON.parse(fs.readFileSync(fxPath, "utf8"));
    for (const k of ["programVK", "rootCVadcopFinal", "publicValues", "proofBytes"]) {
      if (typeof f[k] !== "string" || !f[k].startsWith("0x")) {
        throw new Error(`Fixture field ${k} must be a 0x-prefixed hex string`);
      }
    }

    const Verifier = await ethers.getContractFactory(contract);
    const verifier = await Verifier.deploy();
    await verifier.waitForDeployment();

    // Catch the most common failure mode early: a proof wrapped against a different
    // SNARK proving key than the one the verifier was generated for. The fixture is
    // a leaf, so its root must be the contract's; a mismatch would otherwise surface only
    // as a confusing InvalidProof.
    const onchainRootC = await verifier.getRootCVadcopFinal();
    expect(onchainRootC.toLowerCase()).to.equal(
      f.rootCVadcopFinal.toLowerCase(),
      "rootCVadcopFinal in the fixture does not match the value hardcoded in ZiskVerifier.sol"
    );

    // A real integration passes a constant of its own contract here, not a value read
    // off the proof -- the leaf root above, or its recurser's verkey for an aggregate.
    await expect(
      verifier.verifySnarkProof(
        f.programVK,
        onchainRootC,
        f.publicValues,
        f.proofBytes
      )
    ).to.not.be.reverted;
  });
});
