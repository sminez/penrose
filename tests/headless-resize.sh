#!/usr/bin/env bash
#
# Assert that a window which changes size also changes position.
#
# Usage: tests/headless-resize.sh
#
# A window's size is manage state and its position is render state, so one
# `position_client` call feeds two halves of the plan that are transmitted in
# different sequences. Nothing else in this suite moves a window after it opens,
# which is how a bug lived here: the two halves were written under one `if`, and
# `||` does not evaluate its right operand when the left is true, so every window
# whose size changed kept its old position. Sizes swapped, positions did not, and
# the layout stayed wrong until some later refresh happened to leave the size
# alone -- which is what moving the pointer across a window does.
#
# The run is examples/river_resize: two windows, focus the small one, put it in
# master. What is asserted is that the result tiles -- each window starts where
# the one to its left ends, and together they fill the screen -- which is false
# for either half being stale and true for both being fresh, whatever the layout.
#
# See headless-river.sh for the environment this all has to run in.

set -uo pipefail

cd "$(dirname "$0")/.."

if ! command -v river >/dev/null; then
    echo "headless-resize: river is not installed; skipping" >&2
    exit 77
fi

if ! command -v setxkbmap >/dev/null || ! command -v xkbcomp >/dev/null; then
    echo "headless-resize: setxkbmap and xkbcomp are needed for a keymap; skipping" >&2
    exit 77
fi

CLIENT=""
for c in foot alacritty kitty weston-terminal; do
    command -v "$c" >/dev/null && { CLIENT=$c; break; }
done
if [ -z "$CLIENT" ]; then
    echo "headless-resize: no wayland client to open a window with; skipping" >&2
    exit 77
fi

SPEC=${PENROSE_RIVER_RESIZE_SPEC:-target/debug/examples/river_resize}
if [ ! -x "$SPEC" ]; then
    echo "headless-resize: no build found at $SPEC; run" >&2
    echo "  cargo build --no-default-features --features river --example river_resize" >&2
    exit 1
fi
case "$SPEC" in /*) ;; *) SPEC=$PWD/$SPEC ;; esac

RT=$(mktemp -d /tmp/pw.XXXXXX)
chmod 700 "$RT"
LOG=$RT/river.log
WMLOG=$RT/wm.log
KEYMAP=$RT/keymap.xkb
trap 'rm -rf "$RT"' EXIT

if ! setxkbmap -print -layout us > "$RT/keymap.desc" 2>/dev/null \
   || ! xkbcomp -xkb -o "$KEYMAP" "$RT/keymap.desc" 2>/dev/null; then
    echo "headless-resize: unable to build a keymap; skipping" >&2
    exit 77
fi

# Trace, because what is asserted is what was sent: the proposed dimensions and
# the positions are logged at that level and nowhere else.
cat > "$RT/init.sh" <<EOF
#!/bin/sh
PENROSE_KEYMAP="$KEYMAP" PENROSE_NO_STARTUP_HOOK=1 \
    RUST_LOG=\${RUST_LOG:-penrose=trace} "$SPEC" > "$WMLOG" 2>&1 &
sleep 2
$CLIENT >> "$WMLOG" 2>&1 &
sleep 3
$CLIENT >> "$WMLOG" 2>&1 &
# The spec sleeps 9s, focuses the other window, then swaps it into master.
sleep 20
EOF
chmod +x "$RT/init.sh"

echo "headless-resize: spec=$SPEC client=$CLIENT"

timeout 60 env \
    XDG_RUNTIME_DIR="$RT" \
    WLR_BACKENDS=headless \
    WLR_LIBINPUT_NO_DEVICES=1 \
    river -log-level debug -no-xwayland -c "$RT/init.sh" > "$LOG" 2>&1

status=0
pass() { echo "  PASS  $1"; }
fail() { echo "  FAIL  $1" >&2; status=1; }

echo
echo "headless-resize: results"

# A config's own logger may keep colours; the fields have to be readable either
# way.
sed 's/\x1b\[[0-9;]*m//g' "$WMLOG" > "$RT/plain.log"

screen_w=$(grep -oE 'screens changed rects=\[Rect \{ x: 0, y: 0, w: [0-9]+' "$RT/plain.log" \
           | grep -oE '[0-9]+$' | tail -1)

# The last thing said about each window, which is the state it was left in.
geometry=$(awk '
    match($0, /proposing dimensions id=[0-9]+ w=[0-9]+/) {
        split(substr($0, RSTART, RLENGTH), f, /[= ]/); w[f[4]] = f[6]
    }
    match($0, /positioning id=[0-9]+ x=-?[0-9]+/) {
        split(substr($0, RSTART, RLENGTH), f, /[= ]/); x[f[3]] = f[5]
    }
    END { for (id in w) if (id in x) printf "%s %s %s\n", x[id], w[id], id }
' "$RT/plain.log" | sort -n)

echo "  ....  x width id"
echo "$geometry" | sed 's/^/        /'

if [ -z "$geometry" ] || [ -z "$screen_w" ]; then
    fail "the window manager reported any geometry at all"
    tail -20 "$RT/plain.log" >&2
    exit 1
fi

[ "$(echo "$geometry" | wc -l)" -ge 2 ] \
    && pass "both windows were placed" \
    || fail "both windows were placed"

# Each window starts where the one to its left ends, and the last one ends at the
# edge of the screen. A stale position breaks the first; a stale size the second.
if echo "$geometry" | awk -v screen="$screen_w" '
    { if ($1 != edge) { bad = 1 } ; edge = $1 + $2 }
    END { exit (bad || edge != screen) ? 1 : 0 }
'; then
    pass "the windows tile edge to edge after the swap (screen $screen_w)"
else
    fail "the windows do not tile: a size or a position is stale (screen $screen_w)"
fi

# The pointer follows the focus, and where it lands says which size was used to
# find the middle of the window. River's account of a window is a sequence behind
# the plan, so a warp computed from it puts the pointer half an old window away
# from the new corner -- which is how a fullscreen toggle used to leave it in the
# middle of the screen rather than in the window.
warp=$(grep -oE 'warping the pointer id=[0-9]+ x=-?[0-9]+' "$RT/plain.log" | tail -1)

if [ -z "$warp" ]; then
    fail "the pointer was warped to the focused window"
else
    echo "  ....  $warp"
    # "warping the pointer id=N x=X" against the "x width id" table above.
    if echo "$geometry" | awk -v warp="$warp" '
        BEGIN { split(warp, f, /[= ]/); want_id = f[5]; got = f[7]; ok = 0 }
        $3 == want_id { ok = (got == $1 + int($2 / 2)) }
        END { exit ok ? 0 : 1 }
    '; then
        pass "the pointer landed in the middle of the window it followed"
    else
        fail "the pointer landed somewhere other than the middle of the window it followed"
    fi
fi

if [ "$status" -ne 0 ]; then
    echo >&2
    echo "--- what was sent, in order ---" >&2
    grep -oE "transmitting (manage|render) plan|proposing dimensions id=[0-9]+ w=[0-9]+|positioning id=[0-9]+ x=-?[0-9]+" \
        "$RT/plain.log" | tail -20 >&2
fi
exit "$status"
