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
        lib = pkgs.lib;
        runtimePath = lib.makeBinPath (with pkgs; [
          bash
          coreutils
          gawk
          gnugrep
          gnused
          sudo
        ]);
        runtimeLibPath = lib.makeLibraryPath (with pkgs; [
          libxkbcommon
          libGL
          wayland
          xorg.libX11
          xorg.libXcursor
          xorg.libXrandr
          xorg.libXi
        ]);
      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "linux-it-guy-toolbox";
          version = "0.1.0";
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
            install -Dm644 apps_config.csv "$out/bin/apps_config.csv"
            install -Dm755 main.sh "$out/bin/main.sh"
            install -Dm755 enable-bluetooth.sh "$out/bin/enable-bluetooth.sh"
            install -Dm755 disable-bluetooth.sh "$out/bin/disable-bluetooth.sh"
            install -Dm755 install-fastfetch.sh "$out/bin/install-fastfetch.sh"
            install -Dm755 install-nala.sh "$out/bin/install-nala.sh"
            install -Dm755 install-powertop.sh "$out/bin/install-powertop.sh"
            install -Dm755 install-stacer.sh "$out/bin/install-stacer.sh"
            install -Dm755 install-swapfix.sh "$out/bin/install-swapfix.sh"
            install -Dm755 install-tlp.sh "$out/bin/install-tlp.sh"
            install -Dm755 update-system.sh "$out/bin/update-system.sh"

            wrapProgram $out/bin/linux-it-guy-toolbox \
              --prefix PATH : ${runtimePath} \
              --prefix LD_LIBRARY_PATH : ${runtimeLibPath}
          '';

          meta = {
            description = "The Linux IT Guy Toolbox";
            mainProgram = "linux-it-guy-toolbox";
          };
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
