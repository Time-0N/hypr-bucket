{
  pkgs ? import <nixpkgs> { },
}:

pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    pkg-config
  ];

  buildInputs = with pkgs; [
    glib
    cairo
    pango
    gdk-pixbuf
    gtk4
    gtk4-layer-shell
  ];
}
