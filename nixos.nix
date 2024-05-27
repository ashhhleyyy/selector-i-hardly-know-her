{ pkgs, lib, config, ... }:

with lib;

let
  cfg = config.services.jack-selector;
  selectorArgs = [
    "--client-name" cfg.clientName
    "--listen-port" cfg.port
    "--channels" cfg.channels
  ] ++ concatMap (i: ["--input" i], cfg.inputs);
in
{
  options.services.jack-selector = {
    enable = mkEnableOption "Jack Selector";
    user = mkOption {
      type = types.str;
      default = "jackaudio";
    };
    inputs = mkOption {
      type = types.listOf types.str;
    };
    clientName = mkOption {
      type = types.str;
      default = "selector";
    };
    channels = mkOption {
      type = types.ints.unsigned;
      default = 2;
    };
    port = mkOption {
      type = types.ints.u16;
      default = 6001;
    };
  };

  config = mkIf cfg.enable {
    systemd.services.jack-selector = {
      description = "Jack source selector";
      wantedBy = [ "jack.service" ];
      after = [ "jack.service" ];

      serviceConfig = {
        Type = "simple";
        ExecStart = "${pkgs.selector}/bin/selector-i-hardly-know-her ${escapeShellArgs selectorArgs}";
        User = "jackaudio";
      };
    };
  };
}
