{
  description = "Automatic idle logout manager for Wayland";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    cargo2nix = {
      url = "github:cargo2nix/cargo2nix/release-0.11.0";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };
  };

  outputs = { self, nixpkgs, flake-utils, cargo2nix }: flake-utils.lib.eachDefaultSystem (system:
    let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ cargo2nix.overlays.default ];
      };

      rustPkgs = pkgs.rustBuilder.makePackageSet {
        rustVersion = "latest";
        rustChannel = "stable";
        packageFun = import ./Cargo.nix;
      };
    in
    rec {
      packages = {
        wayout = (rustPkgs.workspace.wayout { });
        default = packages.wayout;
      };
    }
  );
}
