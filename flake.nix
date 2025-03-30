{
  description = "Yet another async ORM";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";

    fenix.url = "github:nix-community/fenix";

    crane.url = "github:ipetkov/crane";
  };

  outputs =
    {
      self,
      nixpkgs,
      fenix,
      crane,
    }:
    let
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forEachSupportedSystem =
        f:
        nixpkgs.lib.genAttrs supportedSystems (
          system:
          let
            pkgs = import nixpkgs {
              inherit system;
              overlays = [ fenix.overlays.default ];
            };

            rustToolchain = pkgs.fenix.stable.withComponents [
              "cargo"
              "rustc"
              "rustfmt"
              "rust-std"
              "rust-analyzer"
              "clippy"
            ];

            craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;
          in
          f {
            inherit
              pkgs
              rustToolchain
              craneLib
              ;
          }
        );
    in
    {
      devShells = forEachSupportedSystem (
        { pkgs, rustToolchain, ... }:
        {
          default = pkgs.mkShell {
            nativeBuildInputs = [
              rustToolchain
            ];
          };
        }
      );
    };
}
