use anyhow::Result;

#[cfg(feature = "remote")]
pub type Client = zisk_sdk::RemoteClient;

#[cfg(not(feature = "remote"))]
pub type Client = zisk_sdk::EmbeddedClient;

/// Optional knobs for the embedded variant. Ignored under `--features remote`.
#[derive(Default)]
pub struct ClientConfig {
    /// Preload PLONK keys (calls `.plonk()` on the embedded builder).
    pub plonk: bool,
    /// Use `EmbeddedOpts::default().minimal_memory()`.
    pub minimal_memory: bool,
}

/// Build a `ProverClient` whose backend is selected at compile time by the `remote` feature.
///
/// - `--features remote`: reads the coordinator URL from `argv[1]`. Defaults to
///   `http://127.0.0.1:7000` if no arg is given. Prepends `http://` if no scheme is provided and
///   appends `:7000` if no port is given (e.g. `localhost` → `http://localhost:7000`).
/// - default (embedded): builds an `EmbeddedClient`. Picks up `--features asm` and `--features gpu`
///   automatically. The `ClientConfig` knobs select between embedded variants.
pub fn build_client(config: ClientConfig) -> Result<Client> {
    build_client_inner(config)
}

#[cfg(feature = "remote")]
fn build_client_inner(_config: ClientConfig) -> Result<Client> {
    let url = std::env::args().nth(1).map(coordinator_url).unwrap_or_else(default_url);
    zisk_sdk::ProverClient::remote(url).build()
}

#[cfg(feature = "remote")]
const DEFAULT_PORT: u16 = 7000;

#[cfg(feature = "remote")]
fn default_url() -> String {
    format!("http://127.0.0.1:{DEFAULT_PORT}")
}

#[cfg(feature = "remote")]
fn coordinator_url(raw: String) -> String {
    let with_scheme = if raw.contains("://") { raw } else { format!("http://{raw}") };
    let after_scheme = with_scheme.split_once("://").map(|(_, r)| r).unwrap_or(&with_scheme);
    let has_port = if after_scheme.starts_with('[') {
        after_scheme.contains("]:") // IPv6 form
    } else {
        after_scheme.contains(':')
    };
    if has_port {
        with_scheme
    } else {
        format!("{with_scheme}:{DEFAULT_PORT}")
    }
}

#[cfg(not(feature = "remote"))]
fn build_client_inner(config: ClientConfig) -> Result<Client> {
    let mut builder = zisk_sdk::ProverClient::embedded();
    if config.plonk {
        builder = builder.plonk();
    }
    if config.minimal_memory {
        builder = builder.with_embedded_opts(zisk_sdk::EmbeddedOpts::default().minimal_memory());
    }
    #[cfg(feature = "asm")]
    let builder = builder.assembly();
    #[cfg(feature = "gpu")]
    let builder = builder.gpu();
    builder.build()
}
