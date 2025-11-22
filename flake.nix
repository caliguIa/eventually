{
  description = "Simple calendar menubar app for macOS";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs = {
    self,
    nixpkgs,
    rust-overlay,
  }: let
    forAllSystems = nixpkgs.lib.genAttrs ["x86_64-darwin" "aarch64-darwin"];
    nixpkgsFor = forAllSystems (system:
      import nixpkgs {
        inherit system;
        overlays = [rust-overlay.overlays.default];
      });
    cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
  in {
    packages = forAllSystems (
      system: let
        pkgs = nixpkgsFor.${system};
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = ["rust-src" "rust-analyzer"];
        };
      in {
        default = pkgs.rustPlatform.buildRustPackage {
          pname = cargoToml.package.name;
          version = cargoToml.package.version;
          src = self;
          cargoLock = {
            lockFile = ./Cargo.lock;
          };
          nativeBuildInputs = [rustToolchain];
          buildType = "release";
          doCheck = false;
          meta = with pkgs.lib; {
            description = cargoToml.package.description;
            license = licenses.gpl3Plus;
            platforms = platforms.darwin;
            mainProgram = "eventually";
          };
        };
        eventually = self.packages.${system}.default;
      }
    );
    devShells = forAllSystems (
      system: let
        pkgs = nixpkgsFor.${system};
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = ["rust-src" "rust-analyzer"];
        };
      in {
        default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            rustToolchain
            cargo-watch
            cargo-edit
            rust-analyzer
            rustfmt
            clippy
            bacon
          ];
          RUST_BACKTRACE = "1";
          RUST_LOG = "info";
        };
      }
    );
    apps = forAllSystems (system: {
      default = {
        type = "app";
        program = "${self.packages.${system}.default}/bin/eventually";
      };
      eventually = self.apps.${system}.default;
    });
    formatter = forAllSystems (system: nixpkgsFor.${system}.nixpkgs-fmt);
  };
}
