// TODO: add description for file

#![no_std]
#![no_main]

use teensy4_panic as _;

#[rtic::app(device = teensy4_bsp, peripherals = true, dispatchers = [GPT1])]
mod app {
    use bsp::board;
    use bsp::hal::lpuart;
    use teensy4_bsp as bsp;
    // use rtic_sync::{make_channel, channel::*};

    const EXPECTED_COMMAND_SIZE: usize = 25;
    const ARRAY_SIZE: usize = 64;

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

    // TODO: should we just call wfi() inside idle for potential power saving?
    #[idle(local = [led, count])]
    fn idle(_cx: idle::Context) -> ! { 
        
        //let count = cx.local.count; // returns a mutable reference to count
        //let led = cx.local.led;

        loop {

            // toggle led every time it enters the idle task
            //led.toggle();
            
            //log::info!("idle? {}", *count);

            // what exactly happens here? Does it skip? Does it block? Does it turn MCU to sleep mode?
            cortex_m::asm::wfi();
            
            //*count += 1;
        }
    }

    // create a hardware interrupt task
    #[task(binds = LPUART2, priority = 3, local = [lpuart2], shared = [buffer, index])]
    fn lpuart2_interrupt(cx: lpuart2_interrupt::Context) {
        let lpuart2 = cx.local.lpuart2;
        let buffer = cx.shared.buffer;
        let index = cx.shared.index;

        // lock shared structs!
        (buffer, index).lock(|buffer, index| {
            // read the byte currently in the data register
            // return type of read_data is ReadData(u32)
            let data = lpuart2.read_data();

            // extract data byte from ReadData and store byte into buffer   
            buffer[*index] = u8::from(data);

            // increment index
            *index = (*index + 1) % ARRAY_SIZE;
                        
            // checks if we have received the EXPECTED COMMAND_SIZE
            if *index == EXPECTED_COMMAND_SIZE {
                parse_message::spawn().unwrap();
            }
        });
    }

    #[task(shared = [buffer, index], local = [output], priority = 1)]
    fn parse_message(cx: parse_message::Context) {

        // TODO: add actual parsing and execute different actions based on command
        let buffer = cx.shared.buffer;
        let output = cx.local.output;
        let index = cx.shared.index;

        // acquire a lock for the shared buffer
        (index, buffer).lock(|index, buffer| {
            // copy the message from buffer!
            for i in 0..ARRAY_SIZE {
                // break early if we reach a "null" value
                // 0x0 is regarded as null in UTF-8 encoding
                if buffer[i] == 0 {
                    break;
                }

                // copy the data
                output[i] = buffer[i];
                buffer[i] = 0;
            }

            // reset index
            *index = 0;
        });

        // write output into the log
        for letter in output {
            
            if (*letter) == 0 {
                break;
            }

            log::info!("{}", *letter as char);
        }
    }

}