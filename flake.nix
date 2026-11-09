{
  description = "USB event collection tool for Linux";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname   = "usblog";
          version = "0.2.0";

          src = ./.;

          cargoLock.lockFile = ./Cargo.lock;

          meta = with pkgs.lib; {
            description = "USB event collection tool for Linux";
            license     = licenses.mit;
            maintainers = [];
            platforms   = platforms.linux;
          };
        };

        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            cargo
            rustc
            rustfmt
            clippy
            rust-analyzer
          ];
        };
      });
}
