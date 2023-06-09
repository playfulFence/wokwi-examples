#![no_std]
#![no_main]

use esp32s2_hal::{
    clock::ClockControl,
    peripherals::Peripherals,
    gpio::*,
    prelude::*,
    systimer::SystemTimer,
    timer::TimerGroup,
    Rtc,
    IO,
    Delay,
};

use keypad2::{Keypad, Columns, Rows};
use esp_println::println;
use esp_backtrace as _;

extern crate alloc;
#[global_allocator]
static ALLOCATOR: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

fn init_heap() {
    const HEAP_SIZE: usize = 32 * 1024;

    extern "C" {
        static mut _heap_start: u32;
    }

    unsafe {
        let heap_start = &_heap_start as *const _ as usize;
        ALLOCATOR.init(heap_start as *mut u8, HEAP_SIZE);
    }
}

use alloc::string::String;
use alloc::string;

#[entry]
fn main() -> ! {
    init_heap();
    let peripherals = Peripherals::take();
    let mut system = peripherals.SYSTEM.split();
    let clocks = ClockControl::boot_defaults(system.clock_control).freeze();

    // Disable the RTC and TIMG watchdog timers
    let mut rtc = Rtc::new(peripherals.RTC_CNTL);
    let timer_group0 = TimerGroup::new(
        peripherals.TIMG0,
        &clocks,
        &mut system.peripheral_clock_control,
    );
    let mut wdt0 = timer_group0.wdt;
    let timer_group1 = TimerGroup::new(
        peripherals.TIMG1,
        &clocks,
        &mut system.peripheral_clock_control,
    );
    let mut wdt1 = timer_group1.wdt;


    rtc.rwdt.disable();
    wdt0.disable();
    wdt1.disable();
    let mut delay = Delay::new(&clocks);

    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);

    let pwd : String = String::from("12345");

    let mut keypad = Keypad::new((  
                    /* ROWS */
        io.pins.gpio6.into_pull_up_input(), // R1
        io.pins.gpio5.into_pull_up_input(), // R2
        io.pins.gpio4.into_pull_up_input(), // R3
        io.pins.gpio3.into_pull_up_input(), // R4
    ),
    (   
                    /* COLUMNS */
        io.pins.gpio2.into_open_drain_output(), // COL1
        io.pins.gpio1.into_open_drain_output(), // COL2
        io.pins.gpio0.into_open_drain_output(), // COL3
    ));

    let mut user_pwd: String = String::new();

    loop {
        let mut key = keypad.read_char(&mut delay);

        if key != ' '
        {
            /* Use # as an Enter button */
            if key == '#' {
                if user_pwd == pwd
                {
                    /* Some additional callback here */
                    println!("Correct!");
                    user_pwd.clear();
                }
                else 
                {
                    user_pwd.clear();
                    println!("Wrong password! Try again...");
                }
            }
            else 
            {
                println!("{}", key);
                user_pwd.push(key);
                
            }
            while (key != ' ') { key = keypad.read_char(&mut delay);}
        }
    }
}
