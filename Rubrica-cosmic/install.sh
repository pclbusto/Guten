#!/usr/bin/env bash
set -euo pipefail

APP_ID="com.gutenair.RubricaCosmic"
PROJECT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
DESKTOP_SOURCE="$PROJECT_DIR/$APP_ID.desktop"
DEFAULT_ICON_SOURCE="$PROJECT_DIR/resources/$APP_ID.png"
ICON_SOURCE="$DEFAULT_ICON_SOURCE"
RELEASE_BIN="$PROJECT_DIR/target/release/rubrica-cosmic"
BIN_DIR="${HOME}/.local/bin"
APPLICATIONS_DIR="${XDG_DATA_HOME:-${HOME}/.local/share}/applications"
ICON_ROOT="${XDG_DATA_HOME:-${HOME}/.local/share}/icons/hicolor"
INSTALLED_BIN="$BIN_DIR/rubrica-cosmic"
INSTALLED_DESKTOP="$APPLICATIONS_DIR/$APP_ID.desktop"

usage() {
    cat <<'EOF'
Uso: ./install.sh [opción] [imagen]

Opciones:
  --full,  -f   Compila en modo release e instala todo (predeterminado).
  --copy,  -c   Instala usando el binario release ya compilado.
  --icons, -i   Escala e instala una imagen y actualiza la caché.
  --help,  -h   Muestra esta ayuda.

Ejemplos:
  ./install.sh
  ./install.sh --full
  ./install.sh --copy
  ./install.sh --icons
  ./install.sh --icons ./otro-icono.png
EOF
}

require_command() {
    if ! command -v "$1" >/dev/null 2>&1; then
        echo "Falta el comando requerido: $1" >&2
        exit 1
    fi
}

validate_sources() {
    if [[ ! -f "$DESKTOP_SOURCE" ]]; then
        echo "No se encontró $DESKTOP_SOURCE" >&2
        exit 1
    fi
    if [[ ! -f "$ICON_SOURCE" ]]; then
        echo "No se encontró $ICON_SOURCE" >&2
        exit 1
    fi
}

build_release() {
    require_command cargo
    echo "Compilando Rúbrica en modo release..."
    CARGO_TARGET_DIR="$PROJECT_DIR/target" cargo build \
        --manifest-path "$PROJECT_DIR/Cargo.toml" \
        --package rubrica-cosmic \
        --release \
        --locked
}

install_application() {
    require_command install
    require_command sed

    if [[ ! -x "$RELEASE_BIN" ]]; then
        echo "No existe el binario release: $RELEASE_BIN" >&2
        echo "Ejecutá primero: ./install.sh --full" >&2
        exit 1
    fi

    echo "Instalando binario..."
    install -Dm755 "$RELEASE_BIN" "$INSTALLED_BIN"

    echo "Instalando acceso de aplicación..."
    sed "s|^Exec=.*|Exec=$INSTALLED_BIN %f|" "$DESKTOP_SOURCE" > "$TMP_DIR/$APP_ID.desktop"
    install -Dm644 "$TMP_DIR/$APP_ID.desktop" "$INSTALLED_DESKTOP"

    if command -v desktop-file-validate >/dev/null 2>&1; then
        desktop-file-validate "$INSTALLED_DESKTOP"
    fi
    if command -v update-desktop-database >/dev/null 2>&1; then
        update-desktop-database "$APPLICATIONS_DIR"
    fi
}

install_icons() {
    require_command install
    require_command gdk-pixbuf-thumbnailer
    require_command file

    echo "Imagen de origen: $ICON_SOURCE"
    echo "Generando e instalando iconos..."
    for size in 32 48 64 128 256 512; do
        resized_icon="$TMP_DIR/$APP_ID-$size.png"
        gdk-pixbuf-thumbnailer -s "$size" "$ICON_SOURCE" "$resized_icon"
        dimensions="$(file -b "$resized_icon")"
        if [[ "$dimensions" != *"$size x $size"* ]]; then
            echo "El icono debe ser cuadrado; el resultado para ${size}px fue: $dimensions" >&2
            exit 1
        fi
        install -Dm644 \
            "$resized_icon" \
            "$ICON_ROOT/${size}x${size}/apps/$APP_ID.png"
    done

    if command -v gtk4-update-icon-cache >/dev/null 2>&1; then
        gtk4-update-icon-cache -f -t "$ICON_ROOT"
    elif command -v gtk-update-icon-cache >/dev/null 2>&1; then
        gtk-update-icon-cache -f -t "$ICON_ROOT"
    else
        echo "Aviso: no se encontró una herramienta para actualizar la caché de iconos." >&2
    fi
}

mode="${1:---full}"

case "$mode" in
    --full|-f)
        if (( $# > 1 )); then
            usage >&2
            exit 2
        fi
        action="full"
        ;;
    --copy|-c)
        if (( $# > 1 )); then
            usage >&2
            exit 2
        fi
        action="copy"
        ;;
    --icons|-i)
        if (( $# > 2 )); then
            usage >&2
            exit 2
        fi
        action="icons"
        ICON_SOURCE="${2:-$DEFAULT_ICON_SOURCE}"
        ;;
    --help|-h)
        if (( $# > 1 )); then
            usage >&2
            exit 2
        fi
        usage
        exit 0
        ;;
    *)
        echo "Opción desconocida: $mode" >&2
        usage >&2
        exit 2
        ;;
esac

validate_sources
require_command mktemp
TMP_DIR="$(mktemp -d)"
trap 'rm -rf -- "$TMP_DIR"' EXIT

case "$action" in
    full)
        build_release
        install_application
        install_icons
        ;;
    copy)
        install_application
        install_icons
        ;;
    icons)
        install_icons
        ;;
esac

echo
case "$action" in
    full)
        echo "Instalación completa finalizada."
        ;;
    copy)
        echo "Archivos instalados sin recompilar."
        ;;
    icons)
        echo "Iconos actualizados."
        ;;
esac

if [[ "$action" != "icons" ]]; then
    echo "Binario: $INSTALLED_BIN"
    echo "Acceso:  $INSTALLED_DESKTOP"
fi

if [[ ":${PATH}:" != *":${HOME}/.local/bin:"* ]]; then
    echo ""
    echo "Aviso: ${HOME}/.local/bin no está en tu PATH."
    echo "Agregalo con: export PATH=\"\$HOME/.local/bin:\$PATH\""
fi

echo "Si COSMIC conserva datos anteriores, cerrá sesión y volvé a entrar."
