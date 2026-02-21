{
  description = "hypr-bucket — lightweight application launcher for Hyprland";

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
          pname = "hypr-bucket";
          version = "1.1.3";

          src = ./.;

          cargoLock.lockFile = ./Cargo.lock;

          nativeBuildInputs = with pkgs; [
            pkg-config
            wrapGAppsHook4
          ];

          buildInputs = with pkgs; [
            gtk4
            gtk4-layer-shell
            glib
            wayland
            libxkbcommon
          ];

          # Needed so gtk4-layer-shell links correctly at runtime
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (with pkgs; [
            gtk4-layer-shell
            wayland
          ]);

          meta = with pkgs.lib; {
            description = "Lightweight and customizable application launcher for Hyprland";
            homepage = "https://github.com/Time-0N/hypr-bucket";
            license = licenses.gpl3Only;
            maintainers = [];
            platforms = platforms.linux;
            mainProgram = "hbucket";
          };
        };

        # `nix run`
        apps.default = flake-utils.lib.mkApp {
          drv = self.packages.${system}.default;
        };

        # `nix develop`
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            cargo
            rustc
            pkg-config
            rust-analyzer
            clippy
            rustfmt
          ];

          buildInputs = with pkgs; [
            gtk4
            gtk4-layer-shell
            glib
            wayland
            libxkbcommon
          ];
        };
      }
    );
}
