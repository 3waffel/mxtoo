{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    utils.url = "github:numtide/flake-utils";
    naersk = {
      url = "github:nix-community/naersk/master";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    utils,
    naersk,
    fenix,
    ...
  }:
    utils.lib.eachDefaultSystem (
      system: let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [
            fenix.overlays.default
            (final: prev: {
              toolchain = with prev.fenix;
                combine [
                  (complete.withComponents [
                    "cargo"
                    "clippy"
                    "rust-src"
                    "rustc"
                    "rustfmt"
                  ])
                  targets.wasm32-unknown-unknown.latest.rust-std
                  targets.x86_64-pc-windows-gnu.latest.rust-std
                ];
            })
          ];
        };
        naersk-lib = with pkgs;
          naersk.lib.${system}.override {
            cargo = toolchain;
            rustc = toolchain;
          };
        nativeBuildInputs = with pkgs; [
          pkg-config
        ];
        buildInputs = with pkgs; [
          openssl
        ];
      in rec {
        packages.mxtoo = naersk-lib.buildPackage {
          pname = "mxtoo";
          src = ./.;
          inherit nativeBuildInputs buildInputs;
          postInstall = ''
            mv public $out
          '';
        };
        packages.default = packages.mxtoo;

        devShells.default = pkgs.mkShell {
          nativeBuildInputs = [pkgs.toolchain];
        };
      }
    )
    // rec {
      nixosModules.mxtoo = {
        config,
        lib,
        pkgs,
        ...
      }: let
        cfg = config.services.mxtoo;
        mxtoo = self.packages.${pkgs.system}.mxtoo;
      in
        with lib; {
          options.services.mxtoo = {
            enable = mkEnableOption "mxtoo service";
            package = mkOption {
              type = types.package;
              default = mxtoo;
            };
            port = mkOption {
              type = types.port;
              default = 7032;
            };
          };

          config = mkIf cfg.enable {
            systemd.services.mxtoo = {
              after = ["network-online.target"];
              wantedBy = ["multi-user.target"];
              environment = {
                MXTOO_PORT = toString cfg.port;
                MXTOO_PUBLIC_DIR = "${cfg.package}/public";
              };
              serviceConfig = {
                DynamicUser = true;
                Restart = "always";
                ExecStart = "${cfg.package}/bin/mxtoo";
              };
            };
          };
        };
      nixosModules.default = nixosModules.mxtoo;
    };
}
