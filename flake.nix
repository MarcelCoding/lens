{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.05";
  };

  outputs = { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
      makeTest = pkgs.callPackage "${nixpkgs}/nixos/tests/make-test-python.nix";

      migration = pkgs.callPackage ./derivation.nix {
        cargoToml = ./lib/migration/Cargo.toml;
      };

      lens = pkgs.callPackage ./derivation.nix {
        cargoToml = ./bin/lens/Cargo.toml;
      };
    in
    {
      checks.${system}.test-sea-orm-cli-migration =
        let
          username = "postgres";
          password = "password";
        in
        makeTest
          {
            name = "test-sea-orm-cli-migration";
            nodes = {
              server = { lib, config, pkgs, ... }: {
                services.postgresql = {
                  enable = true;
                  ensureDatabases = [ username ];
                  ensureUsers = [{
                    name = username;
                    ensureDBOwnership = true;
                  }];
                  initialScript = pkgs.writeScript "initScript" ''
                    ALTER USER postgres WITH PASSWORD '${password}';
                  '';
                };

                systemd.services.postgresql.postStart = lib.mkAfter ''
                  ${migration}/bin/migration refresh --database-url postgresql://${username}:${password}@localhost/${username}
                '';
              };
            };
            testScript = ''
              start_all()
              server.wait_for_unit("postgresql.service")
              server.execute("${pkgs.sea-orm-cli}/bin/sea-orm-cli generate entity --database-url postgresql://${username}:${password}@localhost/${username} --date-time-crate time --with-serde both --output-dir /tmp/out")
              server.copy_from_vm("/tmp/out", "")
            '';
          }
          {
            inherit pkgs;
            inherit (pkgs) system;
          };

      packages.${system} = {
        update-schema = pkgs.writeScriptBin "update-schema" ''
          nix build ${self}#checks.${system}.test-sea-orm-cli-migration
          BUILD_DIR=$(nix build ${self}#checks.${system}.test-sea-orm-cli-migration --no-link --print-out-paths)
          rm -rf ./lib/entity/src/models/*
          cp -r $BUILD_DIR/out/* ./lib/entity/src/models/
          chmod -R 644 ./lib/entity/src/models/*
          ${pkgs.cargo}/bin/cargo fmt
        '';
        inherit lens migration;
      };

      overlays.default = final: prev: {
        inherit (self.packages.${prev.system})
          lens;
      };

      nixosModules = {
        #maid = import ./nixos-module/maid.nix;
        #chef = import ./nixos-module/chef.nix;
      };
    };
}
