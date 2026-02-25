{
  description = "Evergreen note cultivation CLI with spaced repetition";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        packages = {
          default = pkgs.rustPlatform.buildRustPackage {
            pname = "sprout";
            version = "0.1.0";
            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;
            meta = with pkgs.lib; {
              description = "Evergreen note cultivation CLI with spaced repetition";
              homepage = "https://github.com/Yus314/sprout";
              license = licenses.mit;
              mainProgram = "sprout";
            };
          };

          kakoune-sprout = pkgs.kakouneUtils.buildKakounePluginFrom2Nix {
            pname = "kakoune-sprout";
            version = "0.1.0";
            src = ./kak;
            installPhase = ''
              mkdir -p $out/share/kak/autoload/plugins
              cp sprout.kak $out/share/kak/autoload/plugins/sprout.kak
            '';
          };
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            cargo
            rustc
            rust-analyzer
            clippy
            rustfmt
          ];
        };
      }
    );
}
