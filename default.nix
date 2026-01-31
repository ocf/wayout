{ lib, craneLib }:

craneLib.buildPackage {
  pname = "wayout";
  version = "2026-01-30";

  src = craneLib.cleanCargoSource ./.;

  meta = with lib; {
    description = "Automatic idle logout manager for Wayland";
    homepage = "https://github.com/ocf/wayout";
    platforms = platforms.linux;
  };
}
