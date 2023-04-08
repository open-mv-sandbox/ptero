#!/bin/bash
set -e

export MIRIFLAGS="-Zmiri-disable-isolation"

cargo +nightly miri test
