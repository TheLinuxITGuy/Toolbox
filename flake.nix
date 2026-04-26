{
  description = "The Linux IT Guy Toolbox: A Linux software management utility written in Rust";

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

          cargoHash = "sha256-PLLnevbfSVyn2ivfK9PqfuO0cw165KQBY4ZJ1J9GvuQ="; 

          nativeBuildInputs = with pkgs; [ pkg-config makeWrapper ];
          
          buildInputs = with pkgs; [ 
            libxkbcommon
            libGL
            wayland
            # Updated library names to remove deprecation warnings
            xorg.libX11
            xorg.libXcursor
            xorg.libXrandr
            xorg.libXi
          ];

          postInstall = ''
            wrapProgram $out/bin/linux-it-guy-toolbox \
              --prefix LD_LIBRARY_PATH : ${pkgs.lib.makeLibraryPath (with pkgs; [
                libxkbcommon
                libGL
                wayland
                xorg.libX11
                xorg.libXcursor
                xorg.libXrandr
                xorg.libXi
              ])}
          '';
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
