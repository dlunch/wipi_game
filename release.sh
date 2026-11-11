#!/usr/bin/env bash

for platform in ktf lgt
do
    cargo -Zbuild-std=core,alloc build --workspace --target thumbv4t-none-eabi --features $platform --profile release --no-default-features
    # Each game gets its own IDs because the emulator keys save data by them.
    id=0
    for game in rpg puzzle
    do
        cargo run --manifest-path ../wipi/Cargo.toml -p wipi-archiver -- $platform target/thumbv4t-none-eabi/release/$game Clet 0000000$id PD00000$id ./$game/resources > target/${game}_${platform}.zip
        id=$((id + 1))
    done
done
