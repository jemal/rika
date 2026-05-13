{
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";

    flake-utils.url = "github:numtide/flake-utils";

    crane.url = "github:ipetkov/crane";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs:
    inputs.flake-utils.lib.eachSystem
      [
        "x86_64-linux"
      ]
      (
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

          craneLib = (inputs.crane.mkLib pkgs).overrideToolchain rustToolchain;

          cargoToml = fromTOML (builtins.readFile ./Cargo.toml);

          src = craneLib.cleanCargoSource ./.;

          cargoArtifacts = craneLib.buildDepsOnly {
            pname = cargoToml.package.name;
            version = cargoToml.package.version;
            inherit src;
          };

          rika = craneLib.buildPackage {
            pname = cargoToml.package.name;
            version = cargoToml.package.version;
            inherit cargoArtifacts src;
          };

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

          qmlImportPath = "${pkgs.qt6.qtdeclarative}/lib/qt-6/qml:${pkgs.quickshell}/lib/qt-6/qml";

          shellHook = ''
            export RIKA_LAUNCHER_SOCKET="$XDG_RUNTIME_DIR/rika-launcher.sock"
          '';
        in
        {
          packages = {
            inherit rika;
            default = rika;
          };

          checks = {
            inherit rika;
          };

          devShells.default = pkgs.mkShell {
            packages = devPackages;

            inherit shellHook;

            QML_IMPORT_PATH = qmlImportPath;
            QML2_IMPORT_PATH = qmlImportPath;
          };
        }
      );
}
