{ lib, craneLib }:

craneLib.buildPackage {
  pname = "wayout";
  version = "2024-10-22";

  src = craneLib.cleanCargoSource ./.;

  meta = with lib; {
    description = "Automatic idle logout manager for Wayland";
    homepage = "https://github.com/ocf/wayout";
    platforms = platforms.linux;
  };
}
