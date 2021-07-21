#!/usr/bin/env bash
set -ex
for p in near-sdk-macros near-sdk near-contract-standards
do
pushd ./${p}
cargo publish
popd
# Sleep a bit to let the previous package upload to crates.io. Otherwise we fail publishing checks.
sleep 30
done
