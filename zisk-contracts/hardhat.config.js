require("@nomicfoundation/hardhat-toolbox");
const { subtask } = require("hardhat/config");
const {
  TASK_COMPILE_SOLIDITY_GET_SOURCE_PATHS,
  TASK_COMPILE_SOLIDITY_READ_FILE,
} = require("hardhat/builtin-tasks/task-names");
const path = require("path");

// Don't pull stray .sol files out of node_modules (eth-gas-reporter mocks).
subtask(TASK_COMPILE_SOLIDITY_GET_SOURCE_PATHS).setAction(async (_, __, runSuper) => {
  const all = await runSuper();
  const nodeModulesPrefix = path.join(__dirname, "node_modules") + path.sep;
  return all.filter((p) => !p.startsWith(nodeModulesPrefix));
});

// In-memory only patch for IZiskVerifier.sol. The interface declares
// `bytes32 calldata`, which solc rejects (calldata is for dynamic types).
// The fix belongs upstream in the Tera template that generates this file
// (pil2-proofman/setup/stark-recurser/stark2circom/circuit_templates/tera/iverifier.sol.tera).
// We DO NOT touch the on-disk file — the contracts in this directory are
// treated as authoritative artifacts and must remain byte-identical.
subtask(TASK_COMPILE_SOLIDITY_READ_FILE).setAction(async ({ absolutePath }, _, runSuper) => {
  const content = await runSuper({ absolutePath });
  if (path.basename(absolutePath) === "IZiskVerifier.sol") {
    return content.replace(/bytes32\s+calldata\s+/g, "bytes32 ");
  }
  return content;
});

/** @type import('hardhat/config').HardhatUserConfig */
module.exports = {
  solidity: {
    version: "0.8.20",
    settings: {
      optimizer: { enabled: true, runs: 200 }
    }
  },
  paths: {
    sources: ".",
    tests: "./test",
    cache: "./cache",
    artifacts: "./artifacts"
  }
};
