#!/usr/bin/env bash
# Mounts the remote dataset via sshfs (through an SSH master connection that
# the user authenticates interactively if host credentials cannot be reused),
# then starts the YoDa server.
set -euo pipefail

: "${SSH_HOST:?set SSH_HOST to the host alias from your ssh_config}"
: "${REMOTE_IMAGES:?set REMOTE_IMAGES to the remote image folder}"
: "${REMOTE_LABELS:?set REMOTE_LABELS to the remote label folder}"

CTL=/tmp/yoda-ssh-ctl
IMG_MNT=/mnt/images
LBL_MNT=/mnt/labels

# Copy host ssh config so permissions/ownership satisfy ssh without touching the host files.
if [ -d /host-ssh ]; then
  mkdir -p /root/.ssh && cp -rL /host-ssh/. /root/.ssh/ 2>/dev/null || true
  chmod 700 /root/.ssh
  chmod 600 /root/.ssh/* 2>/dev/null || true
fi

SSH_OPTS=(-o ControlPath="$CTL" -o ServerAliveInterval=30 -o ServerAliveCountMax=6)

cleanup() {
  fusermount -u "$IMG_MNT" 2>/dev/null || true
  fusermount -u "$LBL_MNT" 2>/dev/null || true
  ssh "${SSH_OPTS[@]}" -O exit "$SSH_HOST" 2>/dev/null || true
}
trap cleanup EXIT

# 1) Try to reuse existing credentials non-interactively (e.g. host az token cache).
# 2) Otherwise open the master connection interactively so the login challenge can be answered.
if ssh "${SSH_OPTS[@]}" -o BatchMode=yes -o ControlMaster=yes -o ControlPersist=yes -fN "$SSH_HOST" 2>/dev/null; then
  echo "[yoda] Reused existing credentials."
else
  if [ ! -t 0 ]; then
    echo "[yoda] Credentials not reusable and no TTY. Start with 'docker run -it' / 'docker compose run'." >&2
    exit 1
  fi
  echo "[yoda] Authenticating to $SSH_HOST - follow the prompts below."
  ssh "${SSH_OPTS[@]}" -o ControlMaster=yes -o ControlPersist=yes -fN "$SSH_HOST"
fi

mkdir -p "$IMG_MNT" "$LBL_MNT"
SSHFS_OPTS="ssh_command=ssh -o ControlPath=$CTL,reconnect,ServerAliveInterval=30,kernel_cache,cache_timeout=60,max_conns=4"
IMG_OPT="ro"
[ "${IMAGES_READ_WRITE:-0}" = "1" ] && IMG_OPT="rw"

sshfs -o "$SSHFS_OPTS,$IMG_OPT" "$SSH_HOST:$REMOTE_IMAGES" "$IMG_MNT"
sshfs -o "$SSHFS_OPTS" "$SSH_HOST:$REMOTE_LABELS" "$LBL_MNT"
echo "[yoda] Mounted images and labels. Open http://localhost:${YODA_PORT:-8080}"

export YODA_IMAGE_BASE_PATH="$IMG_MNT" YODA_LABEL_BASE_PATH="$LBL_MNT"
# Not exec: keep this shell so the trap unmounts on exit.
/usr/local/yoda-web/yoda-web &
child=$!
trap 'kill $child 2>/dev/null' INT TERM
wait $child
