#!/bin/bash
# Helper script to run the project with correct library paths

export LIBRARY_PATH="/opt/homebrew/lib:$LIBRARY_PATH"
cargo run
