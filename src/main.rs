//! A loopback device. Send characters, and you should see
//! the exact same characters sent back. The LED toggles for
//! every exchanged character.
//!
//! - Pin 14 is the Teensy's TX.
//! - Pin 15 is the Teensy's RX.
//!
//! Baud: 115200bps.

#![no_std]
#![no_main]

use teensy4_panic as _;

#[rtic::app(device = teensy4_bsp, peripherals = true, dispatchers = [GPT1])]
mod app {
    use bsp::board;
    use bsp::hal::lpuart;
    use teensy4_bsp as bsp;
    // use rtic_sync::{make_channel, channel::*};

    const ARRAY_SIZE: usize = 25;

    // indicates size of the channel
    // const CHANNEL_SIZE: usize = 4;

    #[local]
    struct Local {
        led: board::Led,
        lpuart2: board::Lpuart2,

        // counter 
        count: usize,

        // used by the parse task
        output: [u8;ARRAY_SIZE],
    }

    #[shared]
    struct Shared {
        buffer: [u8;ARRAY_SIZE],
        index: usize,
    }

    #[init]
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
        let board::Resources {
            pins,
            lpuart2,
            mut gpio2,
            usb,
            ..
        } = board::t40(cx.device);
        let led = board::led(&mut gpio2, pins.p13);
        led.set();

        bsp::LoggingFrontend::default_log().register_usb(usb);

        let mut lpuart2: board::Lpuart2 = board::lpuart(lpuart2, pins.p14, pins.p15, 115200);
        lpuart2.disable(|lpuart2| {
            lpuart2.disable_fifo(lpuart::Direction::Tx);
            lpuart2.disable_fifo(lpuart::Direction::Rx);
            lpuart2.set_interrupts(lpuart::Interrupts::RECEIVE_FULL);
            lpuart2.set_parity(None);
        });

        let buffer = [0;ARRAY_SIZE];
        let output = [0;ARRAY_SIZE];
        let index = 0;

        // TESTING CHANNEL MESSAGING BETWEEN ASYNC TASKS
        // let (tx,rx) = make_channel!(u8, CHANNEL_SIZE);   
        
        // testing stuff in the idle task
        let count = 0;

        (Shared { buffer, index }, Local { led, lpuart2, output, count }, init::Monotonics())
    }

    #[idle(local = [led, count])]
    fn idle(cx: idle::Context) -> ! { 
        
        let count = cx.local.count; // returns a mutable reference to count
        let led = cx.local.led;

        loop {

            // toggle led every time it enters the idle task
            led.toggle();
            
            log::info!("idle? {}", *count);

            // what exactly happens here? Does it skip? Does it block? Does it turn MCU to sleep mode?
            cortex_m::asm::wfi();
            
            *count += 1;
        }
    }

    #[task(binds = LPUART2, priority = 2, local = [lpuart2], shared = [buffer, index])]
    fn lpuart2_interrupt(cx: lpuart2_interrupt::Context) {
        use lpuart::Status;
        let lpuart2 = cx.local.lpuart2;
        let mut buffer = cx.shared.buffer;
        let mut index = cx.shared.index;

        let status = lpuart2.status();

        // TODO: test whether this is actually needed? I don't think so
        //lpuart2.clear_status(Status::W1C);

        // lock shared structs!
        (buffer, index).lock(|buffer, index| {
            // check the flag that caused the interrupt (we shouldn't need to check this)
            if status.contains(Status::RECEIVE_FULL) {
                // loop until all bytes currently in the fifo is read
                loop {
                    let data = lpuart2.read_data();
                    if data.flags().contains(lpuart::ReadFlags::RXEMPT) {
                        // checks for an RXEMPT flag meaning that there's nothing else to read
                        break;
                    } else {
                        // store byte into buffer
                        buffer[*index] = u8::from(data);

                        // TODO: move parse spawn message to idle block
                        // -> regularly check for index size and call parse when it reaches command value
                        if *index == (ARRAY_SIZE - 1) {
                            parse_message::spawn().unwrap();
                        }

                        // increment index
                        // TODO: increase buffer size
                        // -> this avoids having to mod (%) the index
                        *index = (*index + 1) % ARRAY_SIZE;
                    }
                }
            }
        });
    }

    #[task(shared = [buffer, index], local = [output], priority = 1)]
    fn parse_message(cx: parse_message::Context) {

        // TODO: add actual parsing and execute different actions based on command
        let mut buffer = cx.shared.buffer;
        let mut output = cx.local.output;

        // acquire a lock for the shared buffer
        buffer.lock(|buffer| {
            // copy the message from buffer!
            for i in 0..ARRAY_SIZE {
                output[i] = buffer[i];
                buffer[i] = 0;
            }
        });

        // write output into the log
        for letter in output {
            log::info!("{}", *letter as char);
        }
   

    }

}