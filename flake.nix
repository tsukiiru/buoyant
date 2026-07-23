{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    fenix.url = "github:nix-community/fenix";
  };

  outputs =
    {
      self,
      nixpkgs,
      fenix,
    }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };

      toolchain = fenix.packages.${system}.stable.toolchain;

      fhs = pkgs.buildFHSEnv {
        name = "rust-build-env";
        targetPkgs =
          pkgs: with pkgs; [
            toolchain
            pkg-config
            libX11
            libXcursor
            libXrandr
            libXi
            libxcb
            libxkbcommon
            vulkan-loader
            wayland
            gcc
          ];
        runScript = "bash";
      };
    in
    {
      devShells.${system}.default = fhs.env;
    };
}
