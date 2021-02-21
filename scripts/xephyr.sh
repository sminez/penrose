#!/usr/bin/env bash
CUR_DIR="$(dirname $(readlink -f $0))"
SCREEN_SIZE=${SCREEN_SIZE:-800x600}
XDISPLAY=${XDISPLAY:-:1}
EXAMPLE=${EXAMPLE:-local_test}
# APP=${APP:-st}
APP=${APP:-alacritty}

Xephyr +extension RANDR -screen ${SCREEN_SIZE} ${XDISPLAY} -ac &
XEPHYR_PID=$!

sleep 1
env DISPLAY=${XDISPLAY} "$CUR_DIR/../target/debug/examples/$EXAMPLE" &
WM_PID=$!

trap "kill $XEPHYR_PID && kill $WM_PID" SIGINT SIGTERM exit

env DISPLAY=${XDISPLAY} ${APP} &
wait $WM_PID
kill $XEPHYR_PID
