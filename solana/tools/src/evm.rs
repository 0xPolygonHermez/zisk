//! Measure the unchanged release verifier on an ephemeral, loopback-only Anvil.
use crate::{ensure, root, Result};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    net::TcpListener,
    process::{Child, Command, Stdio},
    thread,
    time::Duration,
};

fn command(program: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(program).args(args).current_dir(root()).output()?;
    ensure(
        output.status.success(),
        &format!("{program} failed: {}", String::from_utf8_lossy(&output.stderr)),
    )?;
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

struct Anvil {
    child: Child,
    url: String,
}

impl Drop for Anvil {
    fn drop(&mut self) {
        // Reap only the process started by this benchmark, including on errors.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Anvil {
    fn start() -> Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port().to_string();
        drop(listener);
        let child = Command::new("anvil")
            .args([
                "--silent",
                "--host",
                "127.0.0.1",
                "--port",
                &port,
                "--chain-id",
                "31337",
                "--hardfork",
                "prague",
                "--base-fee",
                "0",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        let mut node = Self { child, url: format!("http://127.0.0.1:{port}") };
        for _ in 0..100 {
            ensure(node.child.try_wait()?.is_none(), "local Anvil failed to start")?;
            if node.rpc("eth_chainId", json!([])).is_ok() {
                return Ok(node);
            }
            thread::sleep(Duration::from_millis(50));
        }
        Err("local Anvil did not become ready".into())
    }

    fn rpc(&self, method: &str, params: Value) -> Result<Value> {
        // Foundry is already required; cast avoids a second HTTP client dependency.
        let output = command(
            "cast",
            &[
                "rpc",
                "--rpc-url",
                &self.url,
                "--rpc-timeout",
                "5",
                "--raw",
                method,
                &params.to_string(),
            ],
        )?;
        Ok(serde_json::from_str(&output)?)
    }

    fn transact(&self, from: &str, data: &str, to: Option<&str>) -> Result<Value> {
        let mut tx =
            json!({"from": from, "data": data, "gas": "0x989680", "gasPrice": "0x3b9aca00"});
        if let Some(to) = to {
            tx["to"] = to.into();
        }
        let hash = self.rpc("eth_sendTransaction", json!([tx]))?;
        for _ in 0..100 {
            let receipt = self.rpc("eth_getTransactionReceipt", json!([hash]))?;
            if !receipt.is_null() {
                return Ok(receipt);
            }
            thread::sleep(Duration::from_millis(50));
        }
        Err("local transaction was not mined".into())
    }
}

fn text(value: &Value) -> Result<&str> {
    value.as_str().ok_or_else(|| "expected JSON string".into())
}
fn hex_u64(value: &Value) -> Result<u64> {
    Ok(u64::from_str_radix(text(value)?.trim_start_matches("0x"), 16)?)
}
fn bytes(value: &Value) -> Result<Vec<u8>> {
    Ok(hex::decode(text(value)?.trim_start_matches("0x"))?)
}

fn measurement(data: &str, receipt: &Value, expected_status: u64) -> Result<Value> {
    let status = hex_u64(&receipt["status"])?;
    ensure(status == expected_status, "unexpected transaction verdict")?;
    let raw = hex::decode(data.strip_prefix("0x").unwrap_or(data))?;
    let size = raw.len() as u64;
    let zeros = raw.iter().filter(|b| **b == 0).count() as u64;
    let intrinsic = 21_000 + zeros * 4 + (size - zeros) * 16;
    let floor = 21_000 + 10 * (zeros + 4 * (size - zeros));
    let gas = hex_u64(&receipt["gasUsed"])?;
    ensure(gas > floor, "calldata floor binds: subtraction would not isolate execution")?;
    Ok(json!({"transaction_gas": gas, "intrinsic_gas": intrinsic, "execution_gas": gas - intrinsic,
        "calldata_bytes": size, "calldata_zero_bytes": zeros, "calldata_floor_gas": floor, "status": status}))
}

pub fn run() -> Result<()> {
    command("forge", &["build"])?;
    let artifact: Value =
        serde_json::from_slice(&fs::read(root().join("out/ZiskVerifier.sol/ZiskVerifier.json"))?)?;
    let fixture: Value = serde_json::from_slice(&fs::read(root().join("fixtures/blake3.json"))?)?;
    let proof = bytes(&fixture["proofBytes"])?;
    ensure(proof.len() == 6752, "unexpected proof size")?;
    let node = Anvil::start()?;
    ensure(hex_u64(&node.rpc("eth_chainId", json!([]))?)? == 31337, "unexpected chain ID")?;
    let accounts = node.rpc("eth_accounts", json!([]))?;
    let from = text(&accounts[0])?;
    let deployment = node.transact(from, text(&artifact["bytecode"]["object"])?, None)?;
    ensure(hex_u64(&deployment["status"])? == 1, "deployment failed")?;
    let address = text(&deployment["contractAddress"])?;
    let calldata = |proof: &str| {
        command(
            "cast",
            &[
                "calldata",
                "verifySnarkProof(bytes32,bytes32,bytes,bytes)",
                text(&fixture["programVK"])?,
                text(&fixture["rootCVadcopFinal"])?,
                text(&fixture["publicValues"])?,
                proof,
            ],
        )
    };
    let valid = calldata(text(&fixture["proofBytes"])?)?;
    let mut changed = proof.clone();
    changed[512..576].copy_from_slice(&proof[..64]);
    let invalid = calldata(&format!("0x{}", hex::encode(changed)))?;
    let words = proof
        .chunks_exact(32)
        .map(|word| format!("0x{}", hex::encode(word)))
        .collect::<Vec<_>>()
        .join(",");
    let publics: Value = serde_json::from_slice(&fs::read(root().join("fixtures/publics.json"))?)?;
    let public = match &publics[0] {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        _ => return Err("invalid public input".into()),
    };
    let core = command(
        "cast",
        &[
            "calldata",
            "verifyProof(bytes32[211],uint256[1])",
            &format!("[{words}]"),
            &format!("[{public}]"),
        ],
    )?;
    ensure(
        hex_u64(&node.rpc("eth_call", json!([{"to": address, "data": core}, "latest"]))?)? == 1,
        "core call rejected proof",
    )?;
    ensure(
        node.rpc("eth_call", json!([{"to": address, "data": valid}, "latest"]))? == "0x",
        "wrapper call rejected proof",
    )?;
    let mut measurements = Map::new();
    for (name, data, status) in [
        ("valid_wrapper", &valid, 1),
        ("late_invalid_wrapper", &invalid, 0),
        ("valid_core", &core, 1),
    ] {
        let receipt = node.transact(from, data, Some(address))?;
        measurements.insert(name.into(), measurement(data, &receipt, status)?);
    }
    let runtime = bytes(&artifact["deployedBytecode"]["object"])?;
    let report = json!({"runtime": command("anvil", &["--version"])?.lines().next().ok_or("missing Anvil version")?,
        "hardfork": "prague", "compiler": artifact["metadata"]["compiler"], "optimizer": artifact["metadata"]["settings"]["optimizer"],
        "proof_sha256": hex::encode(Sha256::digest(&proof)), "runtime_bytecode_sha256": hex::encode(Sha256::digest(&runtime)),
        "runtime_bytecode_bytes": runtime.len(), "deployment_gas": hex_u64(&deployment["gasUsed"])?, "measurements": measurements});
    let output = serde_json::to_string_pretty(&report)? + "\n";
    fs::write(root().join("benchmarks/evm.json"), &output)?;
    print!("{output}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checks_verdicts_and_calldata_floor() {
        let receipt = json!({"status": "0x1", "gasUsed": "0x61a8"});
        let actual = measurement("0x0001", &receipt, 1).unwrap();
        assert_eq!(actual["intrinsic_gas"], 21020);
        assert_eq!(actual["execution_gas"], 3980);
        assert_eq!(actual["calldata_floor_gas"], 21050);
        assert!(measurement("0x0001", &receipt, 0).is_err());
        assert!(measurement("0x0001", &json!({"status": "0x1", "gasUsed": "0x523a"}), 1).is_err());
        assert!(measurement("0x00", &json!({"status": "0x1"}), 1).is_err());
    }
}
