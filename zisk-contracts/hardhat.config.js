require("@nomicfoundation/hardhat-toolbox");
const { subtask } = require("hardhat/config");
const {
  TASK_COMPILE_SOLIDITY_GET_SOURCE_PATHS,
} = require("hardhat/builtin-tasks/task-names");
const path = require("path");

// Don't pull stray .sol files out of node_modules (eth-gas-reporter mocks).
subtask(TASK_COMPILE_SOLIDITY_GET_SOURCE_PATHS).setAction(async (_, __, runSuper) => {
  const all = await runSuper();
  const nodeModulesPrefix = path.join(__dirname, "node_modules") + path.sep;
  return all.filter((p) => !p.startsWith(nodeModulesPrefix));
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
