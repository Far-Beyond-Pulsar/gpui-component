{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    flake-compat.url = "https://flakehub.com/f/edolstra/flake-compat/1.tar.gz";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs@{ nixpkgs, flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = nixpkgs.lib.systems.flakeExposed;
      perSystem =
        {
          pkgs,
          system,
          ...
        }:
        {
          _module.args.pkgs = import nixpkgs {
            inherit system;
            overlays = [
              (import inputs.rust-overlay)
            ];
          };

          devShells.default =
            let
              toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain;
            in
            pkgs.mkShell {
              nativeBuildInputs = with pkgs; [
                toolchain
                openssl
                protobuf
                rust-analyzer # Used when running
                pkg-config
                alsa-lib
                alsa-utils
                libGL
                wayland
                libxkbcommon
                xorg.libXcursor
                xorg.libXrandr
                xorg.libXi
                xorg.libX11
                xorg.libxcb
              ];
            };
        };
    };
}
