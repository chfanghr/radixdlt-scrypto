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
        # shorthand for accessing outputs
        # you can access crate outputs under `config.nci.outputs.<crate name>` (see documentation)
        outputs = config.nci.outputs;
      in {
        # declare projects
        # relPath is the relative path of a project to the flake root
        # TODO: change this to your crate's path
        nci.projects."radixdlt-scrypto" = {
          relPath = "simulator";
          # export all crates (packages and devshell) in flake outputs
          # alternatively you can access the outputs and export them yourself
          export = true;
        };
        # configure crates
        nci.crates = {
          "simulator" = {
            # look at documentation for more options
          };
        };

        devShells.default = outputs."radixdlt-scrypto".devShell.overrideAttrs (old: {
          packages = (old.packages or []) ++ [pkgs.cmake];
          nativeBuildInputs =
            (old.nativeBuildinputs or [])
            ++ (with pkgs; [
              llvmPackages.libclang
              llvmPackages.libcxxClang
              clang
            ]);
          LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
          BINDGEN_EXTRA_CLANG_ARGS = "-isystem ${pkgs.llvmPackages.libclang.lib}/lib/clang/${lib.getVersion pkgs.clang}/include";
        });

        packages.default = outputs."radixdlt-scrypto".packages.release;
      };
    };
}
