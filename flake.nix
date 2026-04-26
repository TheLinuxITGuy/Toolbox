{
  description = "Toolbox: A Linux software management utility written in Rust";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "linux-it-guy-toolbox";
          version = "2.0.0";
          src = ./.;

          # This hash ensures the build is reproducible. 
          # Set this to lib.fakeSha256 first, run it, and Nix will tell you the real hash.
          cargoHash = "sha256-PLLnevbfSVyn2ivfK9PqfuO0cw165KQBY4ZJ1J9GvuQ="; 

          nativeBuildInputs = with pkgs; [ pkg-config ];
          buildInputs = with pkgs; [ 
            # Add system dependencies here (e.g., openssl, glib, etc.)
          ];
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustc
            cargo
            rust-analyzer
          ];
        };
      });
}
