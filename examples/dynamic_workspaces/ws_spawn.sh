#!/usr/bin/env bash
#------------------
# Select and spawn a workspace type
# jump to that workspace if it already exists
echo -e "1term\n2term\n3term\nweb\nfiles" |
  dmenu -p "WS-SELECT " -l 10
