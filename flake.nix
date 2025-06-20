{
  description = "A flake for csharp-language-server";

  inputs.flake-utils.url = "github:numtide/flake-utils";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        csharp-language-server = pkgs.rustPlatform.buildRustPackage {
          checkFlags = [
            # Test is unable to persist files while testing in nix
            "--skip=first_line_is_jsonrpc"
          ];

          pname = "csharp-language-server";
          version = "0.6.0";

          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = [ pkgs.pkgs.dotnetCorePackages.dotnet_8.sdk ];

        };
      in
      {
        devShell = pkgs.mkShell {
          buildInputs = [ csharp-language-server ];
        };

        packages.csharp-language-server = csharp-language-server;
      }
    );
}
