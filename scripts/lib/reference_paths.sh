#!/usr/bin/env bash
# Shared defaults for local C++ reference trees under reference/ (gitignored content).
# Source from repo scripts: . "$(dirname "$0")/lib/reference_paths.sh" ROOT

reference_paths_init() {
    local root="$1"
    REF="${TFS_REFERENCE_DIR:-$root/reference}"
    CS772="${TFS_CIPSOFT_772_DIR:-$REF/cipsoft-772}"
    TVP772="${TFS_TVP_772_DIR:-$REF/tvp-772}"
    TGM="${TIBIA_GAME_MASTER_DIR:-$CS772/tibia-game-master}"
    QM="${TIBIA_QUERYMANAGER_DIR:-$CS772/tibia-querymanager}"
    LOGIN="${TIBIA_LOGIN_DIR:-$CS772/tibia-login}"
    IPCH="${TIBIA_IPCHANGER_DIR:-$CS772/tibia-ipchanger-master}"
    CIPSOFT_RUNTIME="${TIBIA_GAME_DATA:-$CS772/runtime}"
    CIPSOFT_CLIENT="${TIBIA_CLIENT_DIR:-$CS772/client}"
    CIPSOFT_STATE="${TIBIA_GAME_ONLINE_DIR:-$CS772/state/.tibia-cipsoft}"
    DEFAULT_RSA_PEM="${TIBIA_RSA_PEM:-$CIPSOFT_CLIENT/tibia.pem}"
}

default_cipsoft_game_data() {
    if [[ -n "${TIBIA_GAME_DATA:-}" ]]; then
        echo "$TIBIA_GAME_DATA"
        return 0
    fi
    if [[ -f "$CIPSOFT_RUNTIME/.tibia" && -d "$CIPSOFT_RUNTIME/dat" ]]; then
        echo "$CIPSOFT_RUNTIME"
        return 0
    fi
    # Legacy layout (pre reference/ reorg)
    local root="$1"
    if [[ -f "$root/.tibia" && -d "$root/dat" ]]; then
        echo "$root"
        return 0
    fi
    return 1
}

resolve_rsa_pem_path() {
    local root="$1"
    if [[ -n "${TIBIA_RSA_PEM:-}" ]]; then
        echo "$TIBIA_RSA_PEM"
    elif [[ -f "$DEFAULT_RSA_PEM" ]]; then
        echo "$DEFAULT_RSA_PEM"
    elif [[ -f "$root/tibia.pem" ]]; then
        echo "$root/tibia.pem"
    else
        echo "$TGM/tibia.pem"
    fi
}
