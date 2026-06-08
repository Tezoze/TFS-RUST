#!/usr/bin/env bash
# Shared defaults for local C++ reference trees under reference/ (gitignored content).
# Source from repo scripts: . "$(dirname "$0")/lib/reference_paths.sh" ROOT

reference_paths_init() {
    local root="$1"
    REF="${TFS_REFERENCE_DIR:-$root/reference}"

    if [[ -n "${TFS_REFERENCE_772_DIR:-}" ]]; then
        REF_772="$TFS_REFERENCE_772_DIR"
    elif [[ -n "${TFS_CIPSOFT_772_DIR:-}" ]]; then
        REF_772="$TFS_CIPSOFT_772_DIR"
    elif [[ -d "$REF/classic-772" ]]; then
        REF_772="$REF/classic-772"
    else
        REF_772="$REF/cipsoft-772"
    fi

    # Legacy alias — older scripts still read CS772.
    CS772="$REF_772"
    TVP772="${TFS_TVP_772_DIR:-$REF/tvp-772}"
    TGM="${TIBIA_GAME_MASTER_DIR:-$REF_772/tibia-game-master}"
    QM="${TIBIA_QUERYMANAGER_DIR:-$REF_772/tibia-querymanager}"
    LOGIN="${TIBIA_LOGIN_DIR:-$REF_772/tibia-login}"
    IPCH="${TIBIA_IPCHANGER_DIR:-$REF_772/tibia-ipchanger-master}"
    REF_772_RUNTIME="${TIBIA_GAME_DATA:-$REF_772/runtime}"
    REF_772_CLIENT="${TIBIA_CLIENT_DIR:-$REF_772/client}"
    REF_772_STATE="${TIBIA_GAME_ONLINE_DIR:-$REF_772/state/.tibia-ref-772}"
    if [[ -d "$REF_772/state/.tibia-cipsoft" && ! -d "$REF_772_STATE" ]]; then
        REF_772_STATE="$REF_772/state/.tibia-cipsoft"
    fi
    DEFAULT_RSA_PEM="${TIBIA_RSA_PEM:-$REF_772_CLIENT/tibia.pem}"

    # Deprecated aliases (remove after one release).
    CIPSOFT_RUNTIME="$REF_772_RUNTIME"
    CIPSOFT_CLIENT="$REF_772_CLIENT"
    CIPSOFT_STATE="$REF_772_STATE"
}

default_ref_772_game_data() {
    if [[ -n "${TIBIA_GAME_DATA:-}" ]]; then
        echo "$TIBIA_GAME_DATA"
        return 0
    fi
    if [[ -f "$REF_772_RUNTIME/.tibia" && -d "$REF_772_RUNTIME/dat" ]]; then
        echo "$REF_772_RUNTIME"
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

# Deprecated — use default_ref_772_game_data.
default_cipsoft_game_data() {
    default_ref_772_game_data "$@"
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
