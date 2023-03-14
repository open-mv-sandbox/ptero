#!/bin/bash
set -e

cargo llvm-cov nextest --open
