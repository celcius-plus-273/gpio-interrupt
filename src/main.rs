#![no_std]
#![no_main]

use teensy4_panic as _;

#[rtic::app(device = teensy4_bsp, peripherals = true, dispatchers = [GPT1])]
mod app {
    use teensy4_bsp as bsp;
    use bsp::board;
    use bsp::hal::gpio::{self, Trigger};
    use teensy4_pins::common::*;
    use systick_monotonic::{fugit::Duration, Systick};

    // define some associated types for loca struct definition
    type Led = gpio::Output<P13>;
    type Button = gpio::Input<P9>;

    #[local]
    struct Local {
        led: Led,
    }

    #[shared]
    struct Shared {
        pressed: bool,
        button: Button,
    }

    #[monotonic(binds = SysTick, default = true)]
    type MonoTimer = Systick<1000>;

    #[init]
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
        let board::Resources {
            pins,
            mut gpio2,
            usb,
            ..
        } = board::t40(cx.device);
        
        // usb logging setup
        bsp::LoggingFrontend::default_log().register_usb(usb);

        // Init monotonic systick for delayed spawn
        let mono = Systick::new(cx.core.SYST, 36_000_000);

        let led = gpio2.output(pins.p13);
        let button = gpio2.input(pins.p9);

        gpio2.set_interrupt(&button, Some(Trigger::FallingEdge));

        // set led to off
        led.clear();

        // returned the initialized shared, local, and monotonic resources
        (Shared {pressed: false, button}, Local {led}, init::Monotonics(mono))
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {
            cortex_m::asm::wfi();
        }
    }

    #[task(binds = GPIO2_COMBINED_0_15, local = [led], shared = [pressed, button])]
    fn int_toggle(cx: int_toggle::Context) {
        // reference to shared resource
        let mut pressed = cx.shared.pressed;
        let mut button = cx.shared.button;

        // used for debounce routine 
        // this specifies how long you must wait before being able to press the button again
        let delay_500ms = Duration::<u64, 1, 1000>::from_ticks(5000);

        // MUST clear irq flag
        // If not done then int_toggle becomes an infinite loop
        button.lock(|button| {
            button.clear_triggered();
        });

        // acquire lock for pressed
        pressed.lock(|pressed| {
            // check if button hasn't been pressed
            if *pressed == false {
                // advertise that interrupt was triggered :)
                log::info!("Interrupt was triggered!");
                
                // record that it's been pressed
                *pressed = true;

                // toggle Led
                cx.local.led.toggle();

                // call the debounce routine
                debounce::spawn_after(delay_500ms).unwrap();
            } else {
                // just another debug print
                log::info!("bounce...");
            }
        });
    }

    // debounce routine used to clear the pressed flag after a specified delay
    #[task(shared = [pressed, button])]
    fn debounce(cx: debounce::Context) {
        // get reference to shared resource
        let mut pressed = cx.shared.pressed;
        let mut button = cx.shared.button;

        // debug print
        log::info!("debounced!");

        button.lock(|button| {
            while button.is_set() {
                // this accounts for the button being held high during the debounce routine
            }
        });
        
        // acquire lock and clear button triggered and pressed 
        pressed.lock(|pressed| {
            *pressed = false;
        });
    }
}