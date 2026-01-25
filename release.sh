#!/usr/bin/env bash

cargo -Zbuild-std=core,alloc build --target thumbv4t-none-eabi --features ktf --profile dev --no-default-features
cargo run --manifest-path ../wipi/Cargo.toml -p wipi_archiver -- ktf target/thumbv4t-none-eabi/debug/wipi_game Clet 00000000 PD000000 ./resources > target/wipi_game_ktf.zip

cargo -Zbuild-std=core,alloc build --target thumbv4t-none-eabi --features lgt --profile dev --no-default-features
cargo run --manifest-path ../wipi/Cargo.toml -p wipi_archiver -- lgt target/thumbv4t-none-eabi/debug/wipi_game Clet 00000000 PD000000 ./resources > target/wipi_game_lgt.zip