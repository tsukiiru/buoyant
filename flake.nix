{
  description = "a file explorer for linux";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    { self, nixpkgs, ... }:
    let
      revision = self.shortRev or self.dirtyShortRev or "unknown";
      buoyant-package =
        {
          lib,
          pkg-config,
          rustPlatform,
        }:
        rustPlatform.buildRustPackage {
          pname = "buoyant";
          version = revision;

          src = lib.fileset.toSource {
            root = ./.;
            fileset = lib.fileset.unions [
              ./assets
              ./src
              ./Cargo.toml
              ./Cargo.lock
            ];
          };

          cargoLock = {
            allowBuiltinFetchGit = true;
            lockFile = ./Cargo.lock;
          };

          strictDeps = false;

          nativeBuildInputs = [
            rustPlatform.bindgenHook
            pkg-config
          ];
        };

      inherit (nixpkgs) lib;
      systems = lib.intersectLists lib.systems.flakeExposed lib.platforms.linux;

      forAllSystems = lib.genAttrs systems;
      nixpkgsFor = forAllSystems (system: nixpkgs.legacyPackages.${system});
    in
    {
      devShells = forAllSystems (
        system:
        let
          pkgs = nixpkgsFor.${system};
          rustfmt' = pkgs.rustfmt.override { asNightly = true; };
          inherit (self.packages.${system}) buoyant;
        in
        {
          default = pkgs.mkShell {
            packages = builtins.attrValues {
              inherit (pkgs)
                rustc
                cargo
                clippy
                rust-analyzer
                ;
              inherit rustfmt';
            };

            nativeBuildInputs = with pkgs; [
              rustPlatform.bindgenHook
              pkg-config
              mold
              stdenv.cc.cc.lib
              libX11
              libXcursor
              libXrandr
              libXi
              libxcb
              libxkbcommon
              vulkan-loader
              wayland
              clang
            ];

            RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
            buildInputs = buoyant.buildInputs;
            shellHook = ''
              export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${builtins.toString (pkgs.lib.makeLibraryPath buoyant.buildInputs)}";
            '';
          };
        }
      );

      formatter = forAllSystems (system: nixpkgsFor.${system}.nixfmt);

      packages = forAllSystems (
        system:
        let
          buoyant = nixpkgsFor.${system}.callPackage buoyant-package { };
        in
        {
          inherit buoyant;
          default = buoyant;
        }
      );

      overlays.default = final: _: {
        buoyant = final.callPackage buoyant-package { };
      };
    };
}
