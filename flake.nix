{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      fenix,
      ...
    }:
    let
      system = "x86_64-linux";
      packages.${system}.default = fenix.packages.${system}.minimal.toolchain;
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ fenix.overlays.default ];
      };
      rustToolchain = pkgs.fenix.complete.withComponents [
        "cargo"
        "clippy"
        "rustfmt"
        "rustc"
        "rust-src"
      ];
    in
    {
      devShells.${system}.default =
        with pkgs;
        let
          essentials = [
            pkg-config
            clang
            rustToolchain
            rust-analyzer-nightly
          ];
        in
        mkShell rec {
          buildInputs = essentials ++ [
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
          ];

          shellHook = ''
            export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${builtins.toString (pkgs.lib.makeLibraryPath buildInputs)}";
          '';
        };
    };
}
