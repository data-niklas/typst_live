{
  description = "Simple typst browser editor";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-23.05";

    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flake-utils.url = "github:numtide/flake-utils";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
    };
  };

  outputs = {
    self,
    nixpkgs,
    crane,
    flake-utils,
    rust-overlay,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [rust-overlay.overlays.default];
      };

      lib = pkgs.lib;

      rustToolchain = pkgs.rust-bin.stable.latest.default.override {
        # Set the build targets supported by the toolchain,
        # wasm32-unknown-unknown is required for trunk
        targets = ["wasm32-unknown-unknown"];
      };
      craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

      # When filtering sources, we want to allow assets other than .rs files
      src = lib.cleanSourceWith {
        src = ./.; # The original, unfiltered source
        filter = path: type:
          (lib.hasSuffix "\.html" path)
          || (lib.hasSuffix "\.css" path)
          || (lib.hasSuffix "\.js" path)
          ||
          # Example of a folder for images, icons, etc
          (lib.hasInfix "/assets/" path)
          ||
          # Default filter from crane (allow .rs files)
          (craneLib.filterCargoSources path type);
      };

      # Common arguments can be set here to avoid repeating them later
      commonArgs = {
        inherit src;
        # We must force the target, otherwise cargo will attempt to use your native target
        CARGO_BUILD_TARGET = "wasm32-unknown-unknown";
      };

      # Build *just* the cargo dependencies, so we can reuse
      # all of that work (e.g. via cachix) when running in CI
      cargoArtifacts = craneLib.buildDepsOnly (commonArgs
        // {
          # You cannot run cargo test on a wasm build
          doCheck = false;
        });

      # Build the actual crate itself, reusing the dependency
      # artifacts from above.
      # This derivation is a directory you can put on a webserver.
      typst_live = craneLib.buildTrunkPackage (commonArgs
        // {
          inherit cargoArtifacts;
          nativeBuildInputs = [
            rustToolchain
          ];
        });
    in {
      packages.default = typst_live;
      checks = {};
      devShells.default = pkgs.mkShell {
        inputsFrom = builtins.attrValues self.checks;

        # Extra inputs can be added here
        nativeBuildInputs = with pkgs; [
          rustToolchain
          trunk
        ];
      };
    });
}
