//! The starter code slowly blinks the LED, sets up
//! USB logging, and creates a UART driver using pins
//! 14 and 15. The UART baud rate is [`UART_BAUD`].
//!
//! Despite targeting the Teensy 4.0, this starter code
//! also works on the Teensy 4.1.

#![no_std]
#![no_main]

use bsp::board;
use teensy4_bsp as bsp;
use teensy4_panic as _;

use bsp::hal::timer::Blocking;
use bsp::hal::lpuart::{self, Status};

use core::fmt::Write as _;

/// CHANGE ME to vary the baud rate.
const UART_BAUD: u32 = 115200;
/// Milliseconds to delay before toggling the LED
/// and writing text outputs.
const DELAY_MS: u32 = 100;

#[bsp::rt::entry]
fn main() -> ! {
    // These are peripheral instances. Let the board configure these for us.
    // This function can only be called once!
    let instances = board::instances();

    // Driver resources that are configured by the board. For more information,
    // see the `board` documentation.
    let board::Resources {
        // `pins` has objects that represent the physical pins. The object
        // for pin 13 is `p13`.
        pins,
        // This is a hardware timer. We'll use it for blocking delays.
        mut gpt1,
        // These are low-level USB resources. We'll pass these to a function
        // that sets up USB logging.
        usb,
        // This is the GPIO2 port. We need this to configure the LED as a
        // GPIO output.
        mut gpio2,
        // This resource is for the UART we're creating.
        lpuart2,
        ..
    } = board::t40(instances);

    // When this returns, you can use the `log` crate to write text
    // over USB. Use either `screen` (macOS, Linux) or PuTTY (Windows)
    // to visualize the messages from this example.
    bsp::LoggingFrontend::default_log().register_usb(usb);

    // This configures the LED as a GPIO output.
    //let led = board::led(&mut gpio2, pins.p13);
    let led = gpio2.output(pins.p13);

    // Configures the GPT1 timer to run at GPT1_FREQUENCY. See the
    // constants below for more information.
    gpt1.disable();
    gpt1.set_divider(GPT1_DIVIDER);
    gpt1.set_clock_source(GPT1_CLOCK_SOURCE);

    // Convenience for blocking delays.
    let mut delay = Blocking::<_, GPT1_FREQUENCY>::from_gpt(gpt1);

    // Create the UART driver using pins 14 and 15.
    // Cast it to a embedded_hal trait object so we can
    // use it with the write! macro.
    let mut lpuart2: board::Lpuart2 = board::lpuart(lpuart2, pins.p14, pins.p15, UART_BAUD);
    lpuart2.disable(|lpuart2| {
        //lpuart2.enable_fifo(lpuart::Watermark::tx(8));
        lpuart2.disable_fifo(lpuart::Direction::Tx);
    });

    let word = "+RECV=0,8,COMMANDS,-54,40";

    let mut buffer: [u8;256] = [0;256];
    let mut index: usize = 0;

    let mut word_size = 0;
    for (i,c) in word.chars().enumerate() {
        buffer[i] = c as u8;
        word_size += 1;
    }    



    loop {
        // turn on LED
        led.set();
        
        // check write buffer status
        while !(lpuart2.status().contains(Status::TRANSMIT_EMPTY)) {
            // just loop until transmit data register is empty
            //log::info!("I'm actually waiting lol");
        }

        // write a byte to the uart serial port
        lpuart2.write_byte(buffer[index]);
        
        // write char to the log
        log::info!("sending {}...", buffer[index] as char);

        // long wait if wend of word
        if index == (word_size - 1) {
            log::info!("size is: {:?}", word_size);
            log::info!("{} was sent!", word);
            delay.block_ms(100 * DELAY_MS);
        }

        // led blinks super fast if ongoing write 
        // stays on if on long wait
        led.toggle();

        // increment word buffer index
        // wraps when it reaches buffer size
        index = (index + 1) % word_size;
    }
}

// We're responsible for configuring our timers.
// This example uses PERCLK_CLK as the GPT1 clock source,
// and it configures a 1 KHz GPT1 frequency by computing a
// GPT1 divider.
use bsp::hal::gpt::ClockSource;

/// The intended GPT1 frequency (Hz).
const GPT1_FREQUENCY: u32 = 1_000;
/// Given this clock source...
const GPT1_CLOCK_SOURCE: ClockSource = ClockSource::HighFrequencyReferenceClock;
/// ... the root clock is PERCLK_CLK. To configure a GPT1 frequency,
/// we need a divider of...
const GPT1_DIVIDER: u32 = board::PERCLK_FREQUENCY / GPT1_FREQUENCY;
