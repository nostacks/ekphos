{
  description = "Ekphos - Terminal-based markdown research tool";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        isDarwin = pkgs.stdenv.isDarwin;
        manifest = builtins.fromTOML (builtins.readFile ./Cargo.toml);
      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "ekphos";
          version = manifest.package.version;

          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          cargoBuildFlags = [ "--locked" ];
          cargoTestFlags = [ "--all-targets" "--locked" ];

          nativeBuildInputs = with pkgs; [
            pkg-config
          ];

          buildInputs = with pkgs; [
            # Clipboard support (clipboard-rs)
          ] ++ pkgs.lib.optionals pkgs.stdenv.isLinux [
            libxcb
            libx11
            libxcursor
            libxrandr
            libxi
          ];

          meta = with pkgs.lib; {
            description = "A lightweight, fast, terminal-based markdown research tool";
            homepage = "https://github.com/nostacks/ekphos";
            license = licenses.mit;
            mainProgram = "ekphos";
            platforms = platforms.linux ++ platforms.darwin;
          };
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            cargo
            rustc
            rust-analyzer
            clippy
            rustfmt
            pkg-config
          ] ++ pkgs.lib.optionals pkgs.stdenv.isLinux [
            xorg.libxcb
            xorg.libX11
            xorg.libXcursor
            xorg.libXrandr
            xorg.libXi
          ];
        };
      }
    );
}
