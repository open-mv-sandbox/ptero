#!/bin/bash
set -e

export MIRIFLAGS="-Zmiri-disable-isolation"
export CMD="cargo +nightly miri run --"
export PACK="--package ./packages/dacti-example-web/public/viewer-builtins.dacti-pack"

$CMD create $PACK
$CMD add $PACK --input ./data/shader.wgsl --uuid bacc2ba1-8dc7-4d54-a7a4-cdad4d893a1b
