{
  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  inputs.nci.url = "github:yusdacra/nix-cargo-integration";
  inputs.nci.inputs.nixpkgs.follows = "nixpkgs";
  inputs.parts.url = "github:hercules-ci/flake-parts";
  inputs.parts.inputs.nixpkgs-lib.follows = "nixpkgs";

  outputs = inputs @ {
    parts,
    nci,
    ...
  }:
    parts.lib.mkFlake {inherit inputs;} {
      systems = ["x86_64-linux" "aarch64-darwin"];
      imports = [nci.flakeModule];
      perSystem = {
        config,
        pkgs,
        lib,
        ...
      }: let
        outputs = config.nci.outputs;
        set-stdenv = old: {
          override = old: {stdenv = pkgs.clangStdenv;};
          packages = (old.packages or []) ++ [pkgs.cmake];
        };
      in {
        nci.projects."radixdlt-scrypto" = {
          relPath = "simulator";
          export = true;
        };

        nci.crates = {
          depsOverrides = {inherit set-stdenv;};
          overrides = {inherit set-stdenv;};
        };

        devShells.default = outputs."radixdlt-scrypto".devShell;
      };
    };
}
