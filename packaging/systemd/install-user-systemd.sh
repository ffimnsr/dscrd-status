#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd -- "${SCRIPT_DIR}/../.." && pwd)

UNIT_DIR="${HOME}/.config/systemd/user"
CONFIG_DIR="${HOME}/.config/dscrd-status"
ENV_FILE="${CONFIG_DIR}/dscrd-status.env"
REFRESH_TIME="05:00:00"
ENABLE_TIMER=1
BINARY_PATH=""

usage() {
  cat <<'EOF'
Install dscrd-status user systemd units into ~/.config/systemd/user.

Usage:
  packaging/systemd/install-user-systemd.sh [options]

Options:
  --binary PATH         Path to the dscrd-status binary
  --env-file PATH       Path to the environment file to use
  --refresh-time HH:MM:SS
                        Daily restart time for build-number refresh
  --no-enable-timer     Install the refresh timer but do not enable/start it
  -h, --help            Show this help text
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --binary)
      BINARY_PATH="$2"
      shift 2
      ;;
    --env-file)
      ENV_FILE="$2"
      shift 2
      ;;
    --refresh-time)
      REFRESH_TIME="$2"
      shift 2
      ;;
    --no-enable-timer)
      ENABLE_TIMER=0
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

resolve_binary() {
  if [[ -n "${BINARY_PATH}" ]]; then
    printf '%s\n' "${BINARY_PATH}"
    return
  fi

  if [[ -x "${REPO_ROOT}/target/release/dscrd-status" ]]; then
    printf '%s\n' "${REPO_ROOT}/target/release/dscrd-status"
    return
  fi

  if command -v dscrd-status >/dev/null 2>&1; then
    command -v dscrd-status
    return
  fi

  echo "Could not find dscrd-status binary. Build it first with 'cargo build --release' or pass --binary PATH." >&2
  exit 1
}

validate_time() {
  if [[ ! "$1" =~ ^([01][0-9]|2[0-3]):[0-5][0-9]:[0-5][0-9]$ ]]; then
    echo "Invalid --refresh-time '$1'. Use HH:MM:SS in 24-hour time." >&2
    exit 1
  fi
}

BINARY_PATH=$(resolve_binary)
validate_time "${REFRESH_TIME}"

mkdir -p "${UNIT_DIR}" "${CONFIG_DIR}"

if [[ ! -f "${ENV_FILE}" ]]; then
  install -m 600 "${SCRIPT_DIR}/dscrd-status.env.example" "${ENV_FILE}"
  echo "Created env file at ${ENV_FILE}"
else
  echo "Keeping existing env file at ${ENV_FILE}"
fi

SYSTEMCTL_BIN=$(command -v systemctl)
WORKING_DIR=$(dirname -- "${BINARY_PATH}")

cat > "${UNIT_DIR}/dscrd-status.service" <<EOF
[Unit]
Description=Discord presence keeper
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
WorkingDirectory=${WORKING_DIR}
EnvironmentFile=${ENV_FILE}
ExecStart=${BINARY_PATH}
Restart=always
RestartSec=15
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict

[Install]
WantedBy=default.target
EOF

cat > "${UNIT_DIR}/dscrd-status-refresh.service" <<EOF
[Unit]
Description=Restart dscrd-status to refresh the Discord build number

[Service]
Type=oneshot
ExecStart=${SYSTEMCTL_BIN} --user restart dscrd-status.service
EOF

cat > "${UNIT_DIR}/dscrd-status-refresh.timer" <<EOF
[Unit]
Description=Daily restart timer for dscrd-status

[Timer]
Unit=dscrd-status-refresh.service
OnCalendar=*-*-* ${REFRESH_TIME}
Persistent=true
RandomizedDelaySec=10m

[Install]
WantedBy=timers.target
EOF

systemctl --user daemon-reload
systemctl --user enable --now dscrd-status.service

if [[ "${ENABLE_TIMER}" -eq 1 ]]; then
  systemctl --user enable --now dscrd-status-refresh.timer
fi

cat <<EOF
Installed user units:
  ${UNIT_DIR}/dscrd-status.service
  ${UNIT_DIR}/dscrd-status-refresh.service
  ${UNIT_DIR}/dscrd-status-refresh.timer

Environment file:
  ${ENV_FILE}

Next steps:
  1. Edit ${ENV_FILE} and set DISCORD_TOKEN.
  2. If needed, change the refresh time with:
     systemctl --user edit --full dscrd-status-refresh.timer
  3. Inspect status with:
     systemctl --user status dscrd-status.service
EOF
