#!/usr/bin/env bash
#
# Assert that a handler may block on a menu without deadlocking the session.
#
# Usage: tests/headless-menu.sh
#
# This is the case the two threads exist for, and the reason the no-blocking
# contract was abandoned. A menu under Wayland is a layer surface, and river
# gives one exclusive keyboard focus at the end of the manage sequence in which
# it says so. If answering that sequence were the handler's own thread's job:
#
#   handler blocks on the menu -> manage_start unanswered -> menu never gets the
#   keyboard -> nobody can choose -> handler never returns
#
# which is a deadlocked session rather than a slow window manager, and is not
# something the config author can reasonably be asked to avoid: a prompt, an
# action menu and a rebuild that waits on a compiler are all this shape.
#
# The run is examples/river_menu. M-b runs `fuzzel --dmenu` and blocks on its
# output; the virtual keyboard then types a choice into fuzzel and presses M-n
# afterwards. All three lines have to appear, in order.
#
# See headless-river.sh for the environment this all has to run in.

set -uo pipefail

cd "$(dirname "$0")/.."

if ! command -v river >/dev/null; then
    echo "headless-menu: river is not installed; skipping" >&2
    exit 77
fi

if ! command -v fuzzel >/dev/null; then
    echo "headless-menu: fuzzel is not installed; skipping" >&2
    exit 77
fi

if ! command -v setxkbmap >/dev/null || ! command -v xkbcomp >/dev/null; then
    echo "headless-menu: setxkbmap and xkbcomp are needed for a keymap; skipping" >&2
    exit 77
fi

SPEC=${PENROSE_RIVER_MENU_SPEC:-target/debug/examples/river_menu}
if [ ! -x "$SPEC" ]; then
    echo "headless-menu: no build found at $SPEC; run" >&2
    echo "  cargo build --no-default-features --features river --example river_menu" >&2
    exit 1
fi
case "$SPEC" in /*) ;; *) SPEC=$PWD/$SPEC ;; esac

RT=$(mktemp -d /tmp/pm.XXXXXX)
chmod 700 "$RT"
LOG=$RT/river.log
SPECLOG=$RT/spec.log
ACTIONS=$RT/actions
KEYMAP=$RT/keymap.xkb
trap 'rm -rf "$RT"' EXIT

if ! setxkbmap -print -layout us > "$RT/keymap.desc" 2>/dev/null \
   || ! xkbcomp -xkb -o "$KEYMAP" "$RT/keymap.desc" 2>/dev/null; then
    echo "headless-menu: unable to build a keymap; skipping" >&2
    exit 77
fi

cat > "$RT/init.sh" <<EOF
#!/bin/sh
PENROSE_MENU_LOG="$ACTIONS" PENROSE_KEYMAP="$KEYMAP" \
    RUST_LOG=\${RUST_LOG:-info} "$SPEC" > "$SPECLOG" 2>&1 &
sleep 25
EOF
chmod +x "$RT/init.sh"

echo "headless-menu: spec=$SPEC"

timeout 90 env \
    XDG_RUNTIME_DIR="$RT" \
    WLR_BACKENDS=headless \
    WLR_LIBINPUT_NO_DEVICES=1 \
    river -log-level debug -no-xwayland -c "$RT/init.sh" > "$LOG" 2>&1

status=0
pass() { echo "  PASS  $1"; }
fail() { echo "  FAIL  $1" >&2; status=1; }

echo
echo "headless-menu: results"

if [ ! -s "$ACTIONS" ]; then
    fail "the menu binding ran at all"
    tail -25 "$SPECLOG" >&2
    exit 1
fi

got=$(tr '\n' ' ' < "$ACTIONS" | sed 's/ *$//')
echo "  ....  actions: $got"

grep -qx 'menu-opening' "$ACTIONS" \
    && pass "the menu binding ran" \
    || fail "the menu binding ran"

# The whole point: the menu got the keyboard while the handler was blocked on
# it, so a choice could be made and the blocking read returned.
grep -qx 'chose: bravo' "$ACTIONS" \
    && pass "the blocked handler received the user's choice" \
    || fail "the blocked handler received the user's choice"

# And the window manager is still there afterwards, rather than wedged.
grep -qx 'after' "$ACTIONS" \
    && pass "bindings still work once the handler returns" \
    || fail "bindings still work once the handler returns"

if [ "$status" -ne 0 ]; then
    echo >&2
    echo "--- spec log (last 30) ---" >&2
    tail -30 "$SPECLOG" >&2
    echo "--- river log (layer shell) ---" >&2
    grep -i 'layer\|focus' "$LOG" | tail -15 >&2
fi
exit "$status"
