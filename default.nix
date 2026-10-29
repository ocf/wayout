{ lib, craneLib, makeWrapper, zenity }:

craneLib.buildPackage {
  pname = "wayout";
  version = "2026-01-30";

  src = craneLib.cleanCargoSource ./.;

  nativeBuildInputs = [ makeWrapper ];

  postInstall = ''
    wrapProgram $out/bin/wayout \
      --prefix PATH : ${lib.makeBinPath [ zenity ]}
  '';

  meta = with lib; {
    description = "Automatic idle logout manager for Wayland";
    homepage = "https://github.com/ocf/wayout";
    platforms = platforms.linux;
  };
}
