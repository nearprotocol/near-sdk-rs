#!/bin/bash

if [ -z "$KEEP_NAMES" ]; then
  export RUSTFLAGS="-C link-arg=-s --remap-path-prefix $PWD=/pwd --remap-path-prefix $CARGO_HOME=/cargo_home"
else
  export RUSTFLAGS=''
fi