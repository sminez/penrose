#!/usr/bin/env bash
# Run an example build of penrose in an embeded Xephyr session.
#
# This is intended to be run via the `run-embeded` makefile target
# which will also handle compilation of the examples themselves.
# You will need to have the xephyr utility installed on your system
# for this script to run:
#   https://wiki.archlinux.org/title/Xephyr
#
# usage:
#   EXAMPLE=with_layout_transformers APP=st make run-embeded
#   
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
