{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    {
      nixpkgs,
      ...
    }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };

      fhs = pkgs.buildFHSEnv {
        name = "rust-build-env";
        targetPkgs =
          pkgs: with pkgs; [
            pkg-config
            libxkbcommon
            libxkbcommon.dev
          ];
        multiPkgs =
          pkgs: with pkgs; [
            clang
            zlib
            mold
            stdenv.cc.cc.lib
            libX11
            libXcursor
            libXrandr
            libXi
            libxcb
            vulkan-loader
            wayland
          ];
        runScript = "fish";
      };
    in
    {
      devShells.${system}.default = fhs.env;
    };
}
