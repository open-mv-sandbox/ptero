#!/bin/bash
set -e

cargo build

export TARGET="--target ./packages/dacti-example-web/public/viewer-builtins.dacti-pack"

echo -e "\n# Creating Package"

daicon-tools create $TARGET
daicon-tools set $TARGET --id 0xbacc2ba1 --input ./data/shader.wgsl
daicon-tools set $TARGET --id 0x1f063ad4 --input ./LICENSE-MIT
