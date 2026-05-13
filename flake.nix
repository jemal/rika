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

          lib = pkgs.lib;

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

            meta = {
              description = "Launcher daemon for rika";
              mainProgram = "rika";
            };
          };

          rikaShell = pkgs.stdenvNoCC.mkDerivation {
            pname = "rika-shell";
            version = cargoToml.package.version;

            dontUnpack = true;

            nativeBuildInputs = [
              pkgs.makeWrapper
            ];

            installPhase = ''
              runHook preInstall

              mkdir -p $out/bin $out/share/rika
              cp -r ${./shell} $out/share/rika/shell

              makeWrapper ${lib.getExe pkgs.quickshell} $out/bin/rika-shell \
                --set-default QS_CONFIG_PATH "$out/share/rika/shell" \
                --prefix PATH : ${lib.makeBinPath [ rika ]}

              runHook postInstall
            '';

            meta = {
              description = "Quickshell frontend for rika";
              mainProgram = "rika-shell";
            };
          };

          rikaPackage = pkgs.stdenvNoCC.mkDerivation {
            pname = "rika";
            version = cargoToml.package.version;

            dontUnpack = true;

            installPhase = ''
              runHook preInstall

              mkdir -p $out/bin
              ln -s ${lib.getExe' rika "rika"} $out/bin/rika
              ln -s ${lib.getExe' rikaShell "rika-shell"} $out/bin/rika-shell

              runHook postInstall
            '';

            meta = {
              description = "Launcher daemon and Quickshell frontend for rika";
              mainProgram = "rika-shell";
            };
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
            rika-shell = rikaShell;
            default = rikaPackage;
          };

          checks = {
            inherit rika;
            rika-shell = rikaShell;
            default = rikaPackage;
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
