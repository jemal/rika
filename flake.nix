{
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";

    flake-utils.url = "github:numtide/flake-utils";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs:
    inputs.flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import inputs.nixpkgs {
          inherit system;
        };

        rustToolchain =
          let
            fenix = inputs.fenix.packages.${system};
          in
          fenix.combine [
            fenix.stable.cargo
            fenix.stable.rustc
            fenix.stable.clippy
            fenix.stable.rust-src
            fenix.complete.rustfmt
          ];

        devPackages = [
          rustToolchain

          pkgs.cargo-nextest
          pkgs.rust-analyzer

          pkgs.quickshell
          pkgs.qt6.qtdeclarative
          pkgs.qt6.qttools

          pkgs.desktop-file-utils
          pkgs.jq
          pkgs.socat
          pkgs.xdg-utils
        ];
      in
      {
        devShell = pkgs.mkShell {
          packages = devPackages;

          RIKA_LAUNCHER_SOCKET = "$XDG_RUNTIME_DIR/rika-launcher.sock";
        };

        devShells.default = pkgs.mkShell {
          packages = devPackages;

          RIKA_LAUNCHER_SOCKET = "$XDG_RUNTIME_DIR/rika-launcher.sock";
        };
      }
    );
}
