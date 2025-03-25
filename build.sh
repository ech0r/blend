#!/usr/bin/env bash

# Blend Release Manager build script
# Usage: ./build.sh [dev|prod]
#   dev: Development build (default)
#   prod: Production release build

set -e  # Exit immediately if a command fails

# Default to development mode unless prod is specified
BUILD_MODE="dev"
if [ "$1" == "prod" ]; then
  BUILD_MODE="prod"
  echo "Building in PRODUCTION mode"
else
  echo "Building in DEVELOPMENT mode"
fi

echo "=== Purging old WASM artifacts ==="
find ./static/ -type f -not -name index.html -exec rm {} \;

echo "=== Building frontend ==="
cd frontend

if [ "$BUILD_MODE" == "dev" ]; then
  # Development frontend build
  wasm-pack build --target web --out-dir=../static
else
  # Production frontend build
  wasm-pack build --release --target web --out-dir=../static
fi

cd ..

echo "=== Building backend ==="
if [ "$BUILD_MODE" == "dev" ]; then
  # Development backend build and run
  cargo run
else
  # Production backend build
  cargo build --release
  echo "Production build completed. Run with: ./target/release/blend"
fi
