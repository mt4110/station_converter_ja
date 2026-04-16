{
  description = "station_converter_ja development shell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
      in {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            cargo
            rustc
            rustfmt
            clippy
            rust-analyzer
            openssl
            pkg-config
            nodejs_22
            nodePackages.npm
            git
            jq
            unzip
            docker
            docker-compose
            opentofu
          ];

          shellHook = ''
            alias terraform=tofu
            echo "station_converter_ja dev shell"
            echo "Collima is expected to be installed on macOS outside the repo."
          '';
        };
      });
}
