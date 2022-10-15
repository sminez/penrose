#!/usr/bin/env bash
CUR_DIR="$(dirname $(readlink -f $0))"
SCREEN_SIZE=${SCREEN_SIZE:-800x600}
XDISPLAY=${XDISPLAY:-:1}
EXAMPLE=${EXAMPLE:-minimal}
APP=${APP:-st}

Xephyr +extension RANDR -screen ${SCREEN_SIZE} ${XDISPLAY} -ac &
XEPHYR_PID=$!

sleep 1
env DISPLAY=${XDISPLAY} "$CUR_DIR/../target/debug/examples/$EXAMPLE" &> $CUR_DIR/../xephyr.log &
WM_PID=$!

trap "kill $XEPHYR_PID && kill $WM_PID && rm $CUR_DIR/../xephyr.log" SIGINT SIGTERM exit

env DISPLAY=${XDISPLAY} ${APP} &

touch $CUR_DIR/../xephyr.log
tail -f $CUR_DIR/../xephyr.log

wait $WM_PID
kill $XEPHYR_PID
