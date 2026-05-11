{ config, lib, pkgs, ... }:

let
  cfg = config.services.wayout;
in
{
  options.services.wayout = {
    enable = lib.mkEnableOption "wayout idle logout manager";

    package = lib.mkOption {
      type = lib.types.package;
      default = pkgs.wayout;
      defaultText = lib.literalExpression "pkgs.wayout";
      description = "The wayout package to use.";
    };

    openFirewall = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = "Open TCP port 6767 for the notification HTTP server.";
    };
  };

  config = lib.mkIf cfg.enable {
    systemd.user.services.wayout = {
      description = "Automatic idle logout manager";
      after = [ "graphical-session.target" ];
      partOf = [ "graphical-session.target" ];
      wantedBy = [ "graphical-session.target" ];
      serviceConfig = {
        ExecStart = "${cfg.package}/bin/wayout";
        Type = "simple";
        Restart = "on-failure";
      };
    };

    networking.firewall.allowedTCPPorts = lib.mkIf cfg.openFirewall [ 6767 ];
  };
}
