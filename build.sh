#!/usr/bin/env bash

cd frontend && wasm-pack build --target web --out-dir=../static && cd ..
cargo run
