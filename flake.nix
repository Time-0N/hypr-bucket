{
  description = "hypr-bucket dev environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};

        runtimeLibs = with pkgs; [
          wayland
          libxkbcommon
          fontconfig
          freetype
          vulkan-loader
          mesa
          libx11
          libxcursor
          libxi
          libxrandr
        ];
      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "hypr-bucket";
          version = "1.1.3";

          src = ./.;

          cargoLock.lockFile = ./Cargo.lock;

          nativeBuildInputs = with pkgs; [
            pkg-config
          ];

          buildInputs = runtimeLibs;

          meta = with pkgs.lib; {
            description = "Lightweight and customizable application launcher for Hyprland";
            homepage = "https://github.com/Time-0N/hypr-bucket";
            license = licenses.gpl3Only;
            maintainers = [ Time-0N ];
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

          buildInputs = runtimeLibs;

          shellHook = ''
            export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${pkgs.lib.makeLibraryPath runtimeLibs}"
          '';
        };
      }
    );
}
