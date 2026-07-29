{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    {
      self,
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
            libxkbcommon
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
