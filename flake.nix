{
  description = "";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, crane, flake-utils, ... }:
    {
      overlays.default = final: prev: {
        inherit (self.packages.${prev.system}) selector;
      };
      nixosModules.selector = import ./nixos.nix;
    } // flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
        };

        deps = with pkgs; [
          pkg-config
          jack2
        ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
          pkgs.libiconv
        ];

        craneLib = crane.lib.${system};
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
            rust-analyzer
            mpv
            ffmpeg_6
          ] ++ deps;
        };
      });
}
