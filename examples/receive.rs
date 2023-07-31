// TODO: add description for file

#![no_std]
#![no_main]

use teensy4_panic as _;
pub enum STATE {
    FIRST,
    SECOND,
    THIRD,
    FOURTH,
}

#[rtic::app(device = teensy4_bsp, peripherals = true, dispatchers = [GPT1, GPT2])]
mod app {
    use bsp::board;
    use bsp::hal::lpuart::{self, Status};
    use teensy4_bsp as bsp;
    use crate::STATE;
    // use rtic_sync::{make_channel, channel::*};

    // crates used for delay
    // use rtic_monotonics::systick::*;
    // TODO: MIGRATE TO RTIC v2.0.0

    // systick monotonic sets up clock for delayed spawn calls
    use systick_monotonic::{fugit::Duration, Systick};

    const EXPECTED_COMMAND_SIZE: usize = 25;
    const ARRAY_SIZE: usize = 64;

    // indicates size of the channel
    // const CHANNEL_SIZE: usize = 4;

    #[local]
    struct Local {
        led: board::Led,
        // used by the parse task
        output: [u8;ARRAY_SIZE],
        state: STATE,
    }

    #[shared]
    struct Shared {
        buffer: [u8;ARRAY_SIZE],
        index: usize,
        lpuart2: board::Lpuart2,
    }

    // define monotonic timer block
    #[monotonic(binds = SysTick, default = true)]
    type MonoTimer = Systick<1000>;

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

        // systick monotonic
        let mono = Systick::new(cx.core.SYST, 36_000_000);

        // call blink to test spawn_after
        blink::spawn_after(Duration::<u64, 1, 1000>::from_ticks(10_000)).unwrap();

        // setup state enum :)
        let state = STATE::FIRST;

        // send a message to the radio!
        sequence_call::spawn_after(Duration::<u64, 1, 1000>::from_ticks(30_000)).unwrap(); 
        
        (Shared { buffer, index, lpuart2 }, Local { state, led, output }, init::Monotonics(mono))
    }

    #[idle]
    fn idle(_cx: idle::Context) -> ! { 
        loop {
            // what exactly happens here? Does it skip? Does it block? Does it turn MCU to sleep mode?
            // according to online sources:
            //      WFI(): wait for interrupt
            //      halts execution, puts the core into low power mode, and waits for interrupts
            cortex_m::asm::wfi();
        }
    }

    // create a hardware interrupt task
    #[task(binds = LPUART2, priority = 3, shared = [buffer, index, lpuart2])]
    fn lpuart2_interrupt(cx: lpuart2_interrupt::Context) {
        let lpuart2 = cx.shared.lpuart2;
        let buffer = cx.shared.buffer;
        let index = cx.shared.index;

        // lock shared structs!
        (buffer, index, lpuart2).lock(|buffer, index, lpuart2| {
            // read the byte currently in the data register
            // return type of read_data is ReadData(u32)
            let data = lpuart2.read_data();

            // print whatever you receive
            log::info!("{}", u8::from(data) as char);

            // extract data byte from ReadData and store byte into buffer   
            //buffer[*index] = u8::from(data);

            // increment index
            //*index += 1;
                        
            // checks if we have received the EXPECTED COMMAND_SIZE
            // if *index == EXPECTED_COMMAND_SIZE {
            //     parse_message::spawn().unwrap();
            // }
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

    #[task(shared = [lpuart2])]
    fn talk_to_radio(cx: talk_to_radio::Context, word: &'static str) {
        // take lpuart from shared context
        let mut lpuart2 = cx.shared.lpuart2;

        // CONVERT MESSAGE INTO ARRAY OF CHARS
        let mut message: [u8;64] = [0;64];
        for (i,c) in word.chars().enumerate() {
            message[i] = c as u8;        
            //log::info!("{:#x}", c as u8);
        }
        

        log::info!("Sending...");
        // SEND IT VIA UART
        lpuart2.lock(|lpuart2| {
            for character in message {

                // exit early to avoid sending 0x0
                if character == 0x0 {
                    break;
                }

                // wait for transmit register to be ready
                while !(lpuart2.status().contains(Status::TRANSMIT_EMPTY)) {/*wait for it to be ready*/}
                
                // send via transmit command
                lpuart2.write_byte(character);
            }
        });

        log::info!("Succesfully sent: {}", word);
    }

    #[task(local = [led])]
    fn blink(cx: blink::Context) {
        let led = cx.local.led;
        led.toggle();
        blink::spawn_after(Duration::<u64, 1, 1000>::from_ticks(10_000)).unwrap();
    }

    #[task(local = [state])]
    fn sequence_call(cx: sequence_call::Context) {
        let state = cx.local.state;
        let delay = Duration::<u64, 1, 1000>::from_ticks(80_000);
        match state {
            STATE::FIRST => {
                talk_to_radio::spawn("AT+NETWORKID=2\r\n").unwrap();
                *state = STATE::SECOND;
            }
            STATE::SECOND => {
                talk_to_radio::spawn("AT+ADDRESS=2\r\n").unwrap();
                *state = STATE::THIRD;
            }
            STATE::THIRD => {
                talk_to_radio::spawn("AT+PARAMETER=8,7,4,7\r\n").unwrap();
                *state = STATE::FOURTH;
            }
            STATE::FOURTH => {
                log::info!("waiting for message :D");
            }
        }

        sequence_call::spawn_after(delay).unwrap();        

        /* TODO:
            There seems to be some errors during tx and rx
            Note: header includes CRC check so not sure that the problem is related to the transmission channel

            Possibilities:
            1) Error is caused in the UART channel -> maybe lowering BAUD rate?
            2) Wires could be kinda bad and might be causing noise ont the data
        
         */
    }


}
