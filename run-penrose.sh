#!/usr/bin/env bash

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

# while true; do
  # log out to a file
  "$DIR"/target/debug/penrose &> ~/.penrose.log
# done
