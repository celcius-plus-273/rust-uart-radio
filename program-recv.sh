#!/bin/bash

# This is a shell script that automates the compiling and flashing
# process by runing the cargo objcopy command and then running the
# teensy loader cli tool to program the desired teensy board

# TODO: Implement this autiomation in a runner.rs file for easy
# cargo run integration

file_name="receive.hex" 	# edit this to changee output file name

board="TEENSY41"     	# edit this for to change target board
						# TEENSY40 = Teensy 4.0
						# TEENSY41 = Teensy 4.1
						# https://github.com/PaulStoffregen/teensy_loader_cli

cargo build --release --example receive

# compiles/builds the project and converts the file into an output hex file
rust-objcopy -O ihex target/thumbv7em-none-eabihf/release/examples/receive $file_name

echo "========================================================="
echo ""
echo "     Press pushbutton on Teensy to start programming"
echo ""
echo "========================================================="

# loads the output hex file into the target board (i.e. flashes the mcu)
./teensy_loader_cli --mcu=$board -w $file_name

echo "========================================================="
echo ""
echo "     $file_name was succesfully flashed into $board"
echo ""
echo "========================================================="
