#!/usr/bin/env bash
#
# Assert that key bindings and key sequences work against a real compositor.
#
# Usage: tests/headless-bindings.sh
#
# The question is narrow and the consequence of getting it wrong is not. River
# registers each binding compositor-side and a key sequence disables and
# re-enables them through the plan, so the way this fails is a session in which
# a shortcut silently does nothing -- which is indistinguishable, from inside,
# from a window manager that is working.
#
# The run is examples/river_bindings, which is a window manager rather than a
# client: only a window manager has bindings. It gives the seat a keyboard
# through virtual-keyboard-unstable-v1, because a headless seat has none, and
# then presses:
#
#     M-m   starts a sequence          (the continuation keys go live)
#     a     completes it               -> "seq-a"
#     M-b   an ordinary binding        -> "global-b"
#     M-m   starts one again
#     z     is not in it               -> nothing, and river says ate_unbound_key
#     M-b   again                      -> "global-b"
#
# The two "global-b" lines are the whole point. Either one missing means the
# bindings a capture disabled were not restored.
#
# See headless-river.sh for the environment this all has to run in.

set -uo pipefail

cd "$(dirname "$0")/.."

if ! command -v river >/dev/null; then
    echo "headless-bindings: river is not installed; skipping" >&2
    exit 77   # automake's "skipped" convention
fi

# A virtual keyboard has to hand the compositor a keymap, and xkbcomp will
# produce a self contained one without needing a display.
if ! command -v setxkbmap >/dev/null || ! command -v xkbcomp >/dev/null; then
    echo "headless-bindings: setxkbmap and xkbcomp are needed for a keymap; skipping" >&2
    exit 77
fi

SPEC=${PENROSE_RIVER_BINDINGS_SPEC:-target/debug/examples/river_bindings}
if [ ! -x "$SPEC" ]; then
    echo "headless-bindings: no build found at $SPEC; run" >&2
    echo "  cargo build --no-default-features --features river --example river_bindings" >&2
    exit 1
fi
case "$SPEC" in /*) ;; *) SPEC=$PWD/$SPEC ;; esac

# Short, for the sockaddr_un limit; see headless-river.sh.
RT=$(mktemp -d /tmp/pb.XXXXXX)
chmod 700 "$RT"
LOG=$RT/river.log
SPECLOG=$RT/spec.log
ACTIONS=$RT/actions
KEYMAP=$RT/keymap.xkb
trap 'rm -rf "$RT"' EXIT

if ! setxkbmap -print -layout us > "$RT/keymap.desc" 2>/dev/null \
   || ! xkbcomp -xkb -o "$KEYMAP" "$RT/keymap.desc" 2>/dev/null; then
    echo "headless-bindings: unable to build a keymap; skipping" >&2
    exit 77
fi

cat > "$RT/init.sh" <<EOF
#!/bin/sh
PENROSE_BINDING_LOG="$ACTIONS" PENROSE_KEYMAP="$KEYMAP" \
    RUST_LOG=\${RUST_LOG:-info} "$SPEC" > "$SPECLOG" 2>&1 &
# Long enough for the keyboard's own schedule: it sleeps 4s before it starts and
# about 0.4s between presses.
sleep 20
EOF
chmod +x "$RT/init.sh"

echo "headless-bindings: spec=$SPEC"

timeout 90 env \
    XDG_RUNTIME_DIR="$RT" \
    WLR_BACKENDS=headless \
    WLR_LIBINPUT_NO_DEVICES=1 \
    river -log-level debug -no-xwayland -c "$RT/init.sh" > "$LOG" 2>&1

status=0
pass() { echo "  PASS  $1"; }
fail() { echo "  FAIL  $1" >&2; status=1; }

echo
echo "headless-bindings: results"

if [ ! -s "$ACTIONS" ]; then
    fail "no binding ran at all"
    echo "--- spec log ---" >&2
    tail -25 "$SPECLOG" >&2
    echo "--- river log tail ---" >&2
    tail -25 "$LOG" >&2
    exit 1
fi

got=$(tr '\n' ' ' < "$ACTIONS" | sed 's/ *$//')
echo "  ....  actions: $got"

grep -qx 'seq-a' "$ACTIONS" \
    && pass "a key sequence completes" \
    || fail "a key sequence completes"

# An unbound continuation must not run anything, which is only visible as the
# absence of a second seq-a and the presence of the global that follows it.
[ "$(grep -cx 'seq-a' "$ACTIONS")" -eq 1 ] \
    && pass "an unknown key ends the sequence without running it" \
    || fail "an unknown key ends the sequence without running it"

globals=$(grep -cx 'global-b' "$ACTIONS")
if [ "$globals" -ge 2 ]; then
    pass "ordinary bindings come back after a sequence ($globals/2)"
else
    fail "ordinary bindings come back after a sequence ($globals/2)"
fi

# Ordering, which the counts alone would not catch: a global-b first would mean
# the sequence never armed and M-m ran nothing.
first=$(head -1 "$ACTIONS")
[ "$first" = "seq-a" ] \
    && pass "the sequence armed before the first ordinary binding fired" \
    || fail "the sequence armed before the first ordinary binding fired (first was '$first')"

if [ "$status" -ne 0 ]; then
    echo >&2
    echo "--- spec log (last 30) ---" >&2
    tail -30 "$SPECLOG" >&2
fi
exit "$status"
