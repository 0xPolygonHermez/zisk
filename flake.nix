{
  description = "A flake with project build dependencies";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  inputs.systems.url = "github:nix-systems/default";
  inputs.flake-utils = {
    url = "github:numtide/flake-utils";
    inputs.systems.follows = "systems";
  };

  outputs = { self, nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          config.allowUnfreePredicate = pkg:
            builtins.elem (nixpkgs.lib.getName pkg) [ "mkl" ];
        };
      in {
        devShells.default = pkgs.mkShell {
          packages = [
            pkgs.grpc
            pkgs.gmp
            pkgs.jq
            pkgs.libsodium
            pkgs.libpqxx
            pkgs.libuuid
            pkgs.mkl
            pkgs.openssl
            pkgs.postgresql
            pkgs.protobuf
            pkgs.secp256k1
            pkgs.nlohmann_json
            pkgs.nasm
            pkgs.libgit2
            pkgs.cargo
            pkgs.rustc
          ];
          # Add precompiled library to rustc search path.
          RUSTFLAGS = (builtins.map (a: "-L ${a}/lib") [
            pkgs.libgit2
            "native=${self}/pil2-stark"
          ]);
        };
      });
}
