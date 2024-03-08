{
  description = "Wayland utility to automatically terminate idle user sessions";

  inputs = {
    cargo2nix.url = "github:cargo2nix/cargo2nix/release-0.11.0";
    flake-utils.follows = "cargo2nix/flake-utils";
    nixpkgs.follows = "cargo2nix/nixpkgs";
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
