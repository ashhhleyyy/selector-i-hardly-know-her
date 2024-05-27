{
  description = "";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    rust-overlay.url = "github:oxalica/rust-overlay";

    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, crane, rust-overlay, flake-utils, ... }:
    {
      overlays.default = final: prev: {
        inherit (self.packages.${prev.system}) selector;
      };
      nixosModules.selector = import ./nixos.nix;
    } // flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rust-toolchain = pkgs.rust-bin.stable.latest.default;
        rust-dev-toolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

        deps = with pkgs; [
          pkg-config
          jack2
        ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
          pkgs.libiconv
        ];

        craneLib = (crane.mkLib pkgs).overrideToolchain rust-toolchain;
        selector = craneLib.buildPackage {
          src = craneLib.cleanCargoSource (craneLib.path ./.);

          buildInputs = deps;
        };
      in
      {
        checks = {
          inherit selector;
        };

        packages.selector = selector;
        packages.default = self.packages.${system}.selector;

        apps.default = flake-utils.lib.mkApp {
          drv = selector;
        };

        devShells.default = craneLib.devShell {
          checks = self.checks.${system};

          packages = with pkgs; [
            mpv
            ffmpeg_6
            rust-dev-toolchain
          ] ++ deps;
        };
      });
}
