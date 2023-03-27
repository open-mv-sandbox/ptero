#!/bin/bash
set -e

cargo build

export CMD="./target/debug/ptero-tools-pack"
export TARGET="--target ./packages/dacti-example-web/public/viewer-builtins.dacti-pack"

echo -e "\n# Creating Package"

$CMD create $TARGET
$CMD set $TARGET --id bacc2ba1-8dc7-4d54-a7a4-cdad4d893a1b --input ./data/shader.wgsl

echo -e "\n# Getting Example from Package"

$CMD get $TARGET --id bacc2ba1-8dc7-4d54-a7a4-cdad4d893a1b --output /dev/stdout
