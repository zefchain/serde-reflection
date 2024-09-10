{
  inputs = {
    # <frameworks>
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";

    flake-parts.url = "github:hercules-ci/flake-parts";

    devenv.url = "github:cachix/devenv";
    devenv.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = {self, ...} @ inputs:
    with builtins; let
      lib = inputs.nixpkgs.lib;
    in
      with lib;
        inputs.flake-parts.lib.mkFlake {
          inherit inputs;
          specialArgs = {inherit lib;};
        }
        ({moduleWithSystem, ...}: {
          imports = with inputs; [devenv.flakeModule];
          systems = ["x86_64-linux"];
          perSystem = {
            config,
            system,
            self',
            inputs',
            ...
          }: let
            pkgs = import inputs.nixpkgs {
              inherit system;
              config.allowUnfree = true;
            };
          in {
            _module.args = {inherit pkgs;};
            devenv.shells.default = {config, ...} @ devenvArgs: let
              inherit (config.devenv) root state profile;
            in {
              packages = with pkgs; [nodejs nodePackages.pnpm protobuf_28];
              scripts."gen:proto".exec = concatStringsSep " \\\n" [
                "protoc"
                "--plugin ${root}/node_modules/.bin/protoc-gen-ts_proto"
                "--ts_proto_out src/proto"
                "--ts_proto_opt esModuleInterop=true,snakeToCamel=false,forceLong=number,oneof=unions,outputJsonMethods=false,env=browser"

                "--proto_path schema-proto"
                "--proto_path ${pkgs.protobuf}/include/google/protobuf/"
                ''$(find schema-proto -iname "*.proto")''
              ];
            };
          };
        });
}
