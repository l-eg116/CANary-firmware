# CANary firmware

CANary firmware is the firmware for the CANary project. This project has the goal of making an independant CAN bus listener and emiter.

This firmware was made for an STM32F103CB and bundles a graphical interface, a CAN bus controller and an SD card explorer. A CAN transciever must be used along the STM32 to properly interface with the CAN bus.

## Building the firmware

To build the firmware, ensure you have the `thumbv7m-none-eabi` target installed for your Rust toolchain. This can be done with the following command :

```bash
rustup target add thumbv7m-none-eabi
```

> Note that due to the [rust-toolchain.toml](rust-toolchain.toml) file present in the directory, the target should automatically be installed on building.

You can then build the firmware using cargo :

```bash
cargo build --release
```

> Note : some flags are being set for the `cargo build` command through the [.cargo/config.toml](.cargo/config.toml) file.

The compiled binary can then be found in the `target/thumbv7m-none-eabi/release` directory.

## Fashing the firmware

Flashing makes use of the [probe-rs](https://github.com/probe-rs/probe-rs) tool. First install it using your method of choice by following the [probe-rs installation guide](https://probe.rs/docs/getting-started/installation/).

Then, connect your CANary through Serial Wire Debug (SWD) using the probe of your choice, preferably an ST-Link. More information can be found on the [Probe Setup page](https://probe.rs/docs/getting-started/probe-setup/#st-link) of probe-rs.

Power your CANary by plugging in a USB-C cable and test the connection :

```bash
probe-rs info
```

You should have an output listing the technical characteristics of the micro-controller. If you can an error, read through the error message to try and find the issue. The [probe-rs documentation](https://probe.rs/docs/) can be a great help in troubleshooting.

If all succeeded, you can now flash the firmware by using :

```bash
cargo embed --release
```

> If the firmware wasn't already, it will be built before being flashed onto the micro-controller.
