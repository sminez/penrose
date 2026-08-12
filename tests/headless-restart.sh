#!/usr/bin/env bash
#
# Assert that a restart keeps the session: the clients, the workspace each one
# was on, and which client had focus.
#
# Usage: tests/headless-restart.sh
#
# X11 restarts by exiting and letting a supervisor relaunch, using the X server
# as the state store. River has no property store, so the whole thing rests on
# two claims about river instead, neither of which is in the protocol XML:
#
#   * a hot swap keeps every client alive, so the window manager can be replaced
#     without the session noticing, and
#   * river's window identifier is stable across that swap, so it can be used as
#     the key for what was where.
#
# Both are properties of river rather than of the protocol, which is exactly why
# they are worth asserting rather than assuming.
#
# The run is examples/river_restart. It presses M-S-2 to move the client to tag
# 2, M-2 to look at it, and M-q to restart; the second generation then has to
# find the same window and put it back on tag 2.
#
# See headless-river.sh for the environment this all has to run in.

set -uo pipefail

cd "$(dirname "$0")/.."

if ! command -v river >/dev/null; then
    echo "headless-restart: river is not installed; skipping" >&2
    exit 77
fi

if ! command -v setxkbmap >/dev/null || ! command -v xkbcomp >/dev/null; then
    echo "headless-restart: setxkbmap and xkbcomp are needed for a keymap; skipping" >&2
    exit 77
fi

CLIENT=""
for c in foot alacritty kitty weston-terminal; do
    command -v "$c" >/dev/null && { CLIENT=$c; break; }
done
if [ -z "$CLIENT" ]; then
    echo "headless-restart: no wayland client to open a window with; skipping" >&2
    exit 77
fi

SPEC=${PENROSE_RIVER_RESTART_SPEC:-target/debug/examples/river_restart}
if [ ! -x "$SPEC" ]; then
    echo "headless-restart: no build found at $SPEC; run" >&2
    echo "  cargo build --no-default-features --features river --example river_restart" >&2
    exit 1
fi
case "$SPEC" in /*) ;; *) SPEC=$PWD/$SPEC ;; esac

RT=$(mktemp -d /tmp/pq.XXXXXX)
chmod 700 "$RT"
LOG=$RT/river.log
SPECLOG=$RT/spec.log
STATE=$RT/state
KEYMAP=$RT/keymap.xkb
trap 'rm -rf "$RT"' EXIT

if ! setxkbmap -print -layout us > "$RT/keymap.desc" 2>/dev/null \
   || ! xkbcomp -xkb -o "$KEYMAP" "$RT/keymap.desc" 2>/dev/null; then
    echo "headless-restart: unable to build a keymap; skipping" >&2
    exit 77
fi

# A config of your own is run with its startup hook suppressed. That hook is the
# one part of a real config that reaches outside its own session -- it spawns a
# dozen programs, some of them singletons, and adopts tmux sessions by name -- so
# running it here would disturb the session this is being tested from. Nothing
# under test needs it.
cat > "$RT/init.sh" <<EOF
#!/bin/sh
PENROSE_RESTART_STATE="$STATE" PENROSE_KEYMAP="$KEYMAP" \
    PENROSE_NO_STARTUP_HOOK=1 RUST_LOG=\${RUST_LOG:-info} "$SPEC" > "$SPECLOG" 2>&1 &
sleep 2
$CLIENT >> "$SPECLOG" 2>&1 &
sleep 25
EOF
chmod +x "$RT/init.sh"

echo "headless-restart: spec=$SPEC client=$CLIENT"

timeout 90 env \
    XDG_RUNTIME_DIR="$RT" \
    WLR_BACKENDS=headless \
    WLR_LIBINPUT_NO_DEVICES=1 \
    river -log-level debug -no-xwayland -c "$RT/init.sh" > "$LOG" 2>&1

status=0
pass() { echo "  PASS  $1"; }
fail() { echo "  FAIL  $1" >&2; status=1; }

echo
echo "headless-restart: results"

grep -q 'asking river to stop' "$SPECLOG" \
    && pass "the restart binding ran" \
    || fail "the restart binding ran"

grep -q 'river has finished with us' "$SPECLOG" \
    && pass "river answered the stop with finished" \
    || fail "river answered the stop with finished"

grep -q 'generation=1' "$SPECLOG" \
    && pass "a second generation started" \
    || fail "a second generation started"

# The claim the whole approach rests on: the client outlived the swap and was
# found again by an identifier written down before it.
if grep -q 'managing existing client.*tag=Some("2")' "$SPECLOG"; then
    pass "an existing client came back on the workspace it was on"
else
    fail "an existing client came back on the workspace it was on"
fi

# Which workspace the session comes back up on. Without this the second generation starts on
# whichever tag the client set starts on, so a restart moves the user off the workspace they were
# working on -- and river makes that worse than a default, since it un-hides every window when the
# window manager it was talking to disconnects and then reports whichever one is left under the
# pointer as hovered, which focus-follows-mouse would then follow.
if grep -q 'focusing the client that had focus before the restart' "$SPECLOG"; then
    pass "focus came back to the client that had it"
else
    fail "focus came back to the client that had it"
fi

# And it is still a real window afterwards, not merely remembered.
if grep -A200 'generation=1' "$SPECLOG" | grep -q 'screens changed'; then
    pass "the second generation is running"
else
    fail "the second generation is running"
fi

if [ "$status" -ne 0 ]; then
    echo >&2
    echo "--- state file ---" >&2
    cat "$STATE" 2>/dev/null >&2
    echo "--- spec log ---" >&2
    tail -40 "$SPECLOG" >&2
fi
exit "$status"
