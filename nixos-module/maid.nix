{ pkgs, config, lib, ... }:
let
  cfg = config.dresden-zone.maid;
in
{
  options.dresden-zone.maid = with lib; {
    enable = mkOption {
      type = types.bool;
      default = false;
      description = ''Wether to enable maid'';
    };
    host = mkOption {
      type = types.str;
      default = "127.0.0.1";
      description = ''
        ip address of maid
      '';
    };
    port = mkOption {
      type = types.port;
      default = 8080;
      description = ''
        port address of maid
      '';
    };
    database = {
      host = mkOption {
        type = types.str;
        default = "127.0.0.1";
        description = ''
          Database host
        '';
      };
      port = mkOption {
        type = types.port;
        default = 5354;
        description = ''
          Database port
        '';
      };
      user = mkOption {
        type = types.str;
        default = "maid";
        description = ''
          user for postgres
        '';
      };
      database = mkOption {
        type = types.str;
        default = "tlms";
        description = ''
          postgres database to use
        '';
      };
      passwordFile = mkOption {
        type = types.either types.path types.string;
        default = "";
        description = ''password file from which the postgres password can be read'';
      };
    };
    user = mkOption {
      type = types.str;
      default = "maid";
      description = ''systemd user'';
    };
    group = mkOption {
      type = types.str;
      default = "maid";
      description = ''group of systemd user'';
    };
    log_level = mkOption {
      type = types.str;
      default = "info";
      description = ''log level of the application'';
    };
  };

  config = lib.mkIf cfg.enable {
    systemd = {
      services = {
        "maid" = {
          enable = true;
          wantedBy = [ "multi-user.target" "network.target" ];

          script = ''
            exec ${pkgs.maid}/bin/maid --listend-addr ${cfg.host}:${toString cfg.port}&
          '';

          environment = {
            "RUST_LOG" = "${cfg.log_level}";
            "RUST_BACKTRACE" = if (cfg.log_level == "info") then "0" else "1";
            "POSTGRES_HOST" = "${cfg.database.host}";
            "POSTGRES_PORT" = "${toString cfg.database.port}";
            "POSTGRES_USER" = "${toString cfg.database.user}";
            "POSTGRES_DATABASE" = "${toString cfg.database.database}";
            "POSTGRES_PASSWORD_PATH" = "${cfg.database.passwordFile}";
          };

          serviceConfig = {
            Type = "forking";
            User = cfg.user;
            Restart = "always";
          };
        };
      };
    };

    # user accounts for systemd units
    users.users."${cfg.user}" = {
      name = "${cfg.user}";
      description = "runs maid";
      isNormalUser = false;
      isSystemUser = true;
      group = cfg.group;
      uid = 1502;
    };
  };
}
