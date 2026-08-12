#!/usr/bin/env bash
#
# Run the river backend against a real compositor, with no display and no
# hardware, and assert on what river says happened.
#
# Usage: tests/headless-river.sh [seconds]
#
# Nothing above the protocol bindings can be verified any other way: the
# manage/render sequence loop, the plan, the event ordering and the bindings all
# compile without ever having run.  A unit test cannot help -- there is no
# compositor to talk to and every interesting behaviour is a conversation.
#
# The recipe is xmonad-river's (tests/headless-river.sh there), which learned
# each of these the expensive way:
#
#   * XDG_RUNTIME_DIR must be short.  The Wayland socket path goes in a
#     sockaddr_un, which caps out at 108 bytes; a runtime dir under a deep
#     temporary path silently exhausts every socket name from wayland-0 to
#     wayland-32 and then river exits with AddSocketFailed.
#
#   * -no-xwayland, or river tries to start Xwayland and fails the whole
#     session when it cannot.
#
#   * Subprocess output must go to a file rather than through a pipe.  Anything
#     buffering in a pipeline loses its output when the run is killed, which
#     reads as "the window manager printed nothing" -- and "printed nothing" is
#     also what a healthy window manager does.
#
# == What is asserted
#
# river's debug log is the oracle.  The line that matters is the last one:
#
#   manage sequence finish        a manage sequence completed
#   render sequence finish        a render sequence completed
#   new xdg_toplevel              a client actually asked for a window
#   sent N tracked configure(s)   with N > 0, river accepted propose_dimensions
#                                 and configured a real window
#
# Only the last proves the layout reached the compositor.  The others are true
# of a river with no window manager at all, which is exactly the trap: river
# runs manage and render sequences on its own schedule regardless, so counting
# them proves nothing.

set -uo pipefail

cd "$(dirname "$0")/.."

DURATION=${1:-12}

if ! command -v river >/dev/null; then
    echo "headless-river: river is not installed; skipping" >&2
    exit 77   # automake's "skipped" convention
fi

# A client to open a window with.  Any Wayland client will do; the test only
# needs something that creates an xdg_toplevel.
CLIENT=""
for c in foot alacritty kitty weston-terminal; do
    command -v "$c" >/dev/null && { CLIENT=$c; break; }
done

# A config of your own can be tested by pointing this at its binary, which is
# the point of the exercise: build the config you actually run and run it here.
WM=${PENROSE_RIVER_WM:-target/debug/examples/river_minimal}
if [ ! -x "$WM" ]; then
    echo "headless-river: no river build found at $WM; run" >&2
    echo "  cargo build --no-default-features --features river --example river_minimal" >&2
    exit 1
fi
case "$WM" in /*) ;; *) WM=$PWD/$WM ;; esac

# Short, for the sockaddr_un limit above.
RT=$(mktemp -d /tmp/pr.XXXXXX)
chmod 700 "$RT"
LOG=$RT/river.log
WMLOG=$RT/wm.log
trap 'rm -rf "$RT"' EXIT

cat > "$RT/init.sh" <<EOF
#!/bin/sh
# river runs this instead of the default init.  It starts the window manager,
# which connects back as a client, then clients to give it something to do.
RUST_LOG=\${RUST_LOG:-debug} "$WM" > "$WMLOG" 2>&1 &
sleep 3
${CLIENT:+$CLIENT >> "$WMLOG" 2>&1 &}
sleep 3
${CLIENT:+$CLIENT >> "$WMLOG" 2>&1 &}
sleep $DURATION
EOF
chmod +x "$RT/init.sh"

echo "headless-river: wm=$WM"
echo "headless-river: client=${CLIENT:-<none found>}"

timeout $((DURATION + 20)) env \
    XDG_RUNTIME_DIR="$RT" \
    WLR_BACKENDS=headless \
    WLR_LIBINPUT_NO_DEVICES=1 \
    river -log-level debug -no-xwayland -c "$RT/init.sh" > "$LOG" 2>&1

configures=$(grep -oE 'sent [0-9]+ tracked configure' "$LOG" \
             | grep -oE '[0-9]+' | sort -rn | head -1)
configures=${configures:-0}
toplevels=$(grep -c 'new xdg_toplevel' "$LOG")
manages=$(grep -c 'manage sequence finish' "$LOG")
renders=$(grep -c 'render sequence finish' "$LOG")

status=0
report() {
    if [ "$2" = ok ]; then printf '  PASS  %s\n' "$1"
    else printf '  FAIL  %s\n' "$1" >&2; status=1
    fi
}

echo
echo "headless-river: results"
[ "$manages" -gt 0 ] && report "river completed a manage sequence ($manages)" ok \
                     || report "river completed a manage sequence" no
[ "$renders" -gt 0 ] && report "river completed a render sequence ($renders)" ok \
                     || report "river completed a render sequence" no

# A protocol error disconnects the window manager, so it is both the likeliest
# outcome of a bug in the plan and one that leaves the compositor running and
# looking fine.  Penrose's own ERROR lines are worth failing on too: they are
# how the run loop reports a handler that returned Err.
backend_trouble=$(grep -nE 'ERROR|protocol error|wl_display|invalid object|no such interface' "$WMLOG" 2>/dev/null)
if [ -n "$backend_trouble" ]; then
    report "the window manager reported no errors" no
    echo "--- window manager errors ---" >&2
    echo "$backend_trouble" | head -20 >&2
else
    report "the window manager reported no errors" ok
fi

if grep -q 'sequence_order\|manage sequence.*not started\|render sequence.*not started' "$LOG"; then
    report "no sequence ordering violations" no
    grep -n 'sequence_order' "$LOG" | head -5 >&2
else
    report "no sequence ordering violations" ok
fi

# Not an assertion.  The window manager's own output is expected, but it is the
# first thing wanted when something else fails, so it is shown rather than
# hidden.
if [ -s "$WMLOG" ]; then
    echo
    echo "window manager output ($(wc -l < "$WMLOG") lines, first 15):"
    head -15 "$WMLOG" | sed 's/^/  | /'
fi

if [ -n "$CLIENT" ]; then
    [ "$toplevels" -gt 0 ] && report "a client created a toplevel ($toplevels)" ok \
                           || report "a client created a toplevel" no
    # The one that matters.
    [ "$configures" -gt 0 ] \
        && report "river configured a window from the layout ($configures)" ok \
        || report "river configured a window from the layout (got 0)" no
    # Two windows configured in one sequence is the layout tiling rather than
    # merely placing: both were given geometry from the same run of it.
    [ "$configures" -gt 1 ] \
        && report "the layout tiled two windows at once ($configures)" ok \
        || report "the layout tiled two windows at once (got $configures)" no
fi

if [ "$status" -ne 0 ]; then
    echo >&2
    echo "river log kept at $LOG.keep" >&2
    cp "$LOG" "$LOG.keep" 2>/dev/null
    cp "$WMLOG" "$WMLOG.keep" 2>/dev/null
    trap - EXIT
fi
exit "$status"
