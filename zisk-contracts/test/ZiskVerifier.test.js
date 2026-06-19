const { expect } = require("chai");
const { ethers } = require("hardhat");
const fs = require("fs");
const path = require("path");

describe("ZiskVerifier", function () {
  it("verifies a wrapped PLONK proof on-chain", async function () {
    const fxPath =
      process.env.ZISK_SOLIDITY_FIXTURE ||
      path.join(__dirname, "..", "fixtures", "fixture.json");

    if (!fs.existsSync(fxPath)) {
      throw new Error(
        `Fixture not found at ${fxPath}. ` +
          `Generate one with: cargo zisk export-solidity-calldata -p <plonk.bin> -o ${fxPath}`
      );
    }

    const f = JSON.parse(fs.readFileSync(fxPath, "utf8"));
    for (const k of ["programVK", "rootCVadcopFinal", "publicValues", "proofBytes"]) {
      if (typeof f[k] !== "string" || !f[k].startsWith("0x")) {
        throw new Error(`Fixture field ${k} must be a 0x-prefixed hex string`);
      }
    }

    const Verifier = await ethers.getContractFactory("ZiskVerifier");
    const verifier = await Verifier.deploy();
    await verifier.waitForDeployment();

    // Catch the most common failure mode early: a proof wrapped against a different
    // PLONK proving key than the one PlonkVerifier.sol was generated for. The contract
    // hardcodes its expected vadcop-final root; if it disagrees with the proof's, the
    // verifier would revert with a confusing InvalidProof.
    const onchainRootC = await verifier.getRootCVadcopFinal();
    expect(onchainRootC.toLowerCase()).to.equal(
      f.rootCVadcopFinal.toLowerCase(),
      "rootCVadcopFinal in the fixture does not match the value hardcoded in ZiskVerifier.sol"
    );

    await expect(
      verifier.verifySnarkProof(
        f.programVK,
        f.rootCVadcopFinal,
        f.publicValues,
        f.proofBytes
      )
    ).to.not.be.reverted;
  });
});
