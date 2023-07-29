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

//use core::fmt::Write;

use bsp::hal::timer::Blocking;
use bsp::hal::lpuart::{self, Status};
use core::num::NonZeroU32;

/// CHANGE ME to vary the baud rate.
const UART_BAUD: u32 = 115200;
/// Milliseconds to delay before toggling the LED
/// and writing text outputs.
const DELAY_MS: u32 = 500;

const ARRAY_SIZE: usize = 12;

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
        //lpuart2.enable_fifo(lpuart::Watermark::rx(NonZeroU32::new(1).unwrap()));
        lpuart2.disable_fifo(lpuart::Direction::Rx);
    });
    delay.block_ms(100);

    // print the watermark value

    let mut buffer: [u8;11] = [0;11];
    let mut index: usize = 0;
    let mut output: [char;ARRAY_SIZE] = [' ';ARRAY_SIZE];
    let mut counter: u32 = 0;

    loop {

        delay.block_ms(200);

        log::info!("Watermark value is: {}", lpuart2.fifo_watermark(lpuart::Direction::Rx));
        // turns on LED
        led.toggle();
        log::info!("Cycle: {}", counter);
        // receive from UART Serial Port
        // Does it block until data is available? Or it just sends an error?
        // How do we ensure that there's data to be read?
        let status = lpuart2.status();
        log::info!("          222211111111110000000000");
        log::info!("          321098765432109876543210");
        log::info!("Status: {:#b}", status);

        lpuart2.clear_status(Status::W1C);

        if status.contains(Status::RECEIVE_FULL) {
            // read data until until empty
            loop {
                let data = lpuart2.read_data();
                
                // if data.flags().contains(lpuart::ReadFlags::RXEMPT) {

                //     //log::info!("Broke out!   {}   Status: {:#b}", u8::from(data) as char, data.flags());
                //     break;
                // }

                if index == ARRAY_SIZE {
                    index = 0;
                    break;
                }

                output[index] = u8::from(data) as char;
                index = index + 1;
                //log::info!("             {}   Status: {:#b}", u8::from(data) as char, data.flags());
            }
            log::info!("Read value: [ ");
            log::info!("{:?}", output);
            log::info!("]");
            //lpuart2.flush_fifo(lpuart::Direction::Rx);
            // convert to char and log
            // for (i, c) in buffer.iter().enumerate() {
            //     output[i] = *c as char;
            // }
            // log::info!("Buffer is: {:?}", output);
        } else {
            log::info!("Read value: nothing...");
        }

        // turns off LED
        led.toggle();
        counter = counter.wrapping_add(1);
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
