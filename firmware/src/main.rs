//! Phase 2: interrupt-driven stepper motion and a line-oriented serial console.
//!
//! The ATmega328PB Nano runs the ATmega328P instruction/peripheral subset. Keep
//! that target: the installed avr-gcc 9 cannot link a native 328PB executable.
//! D2..D5 are signals only; motor power comes from external 5 V, common ground.

#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]

use arduino_hal::{pac, prelude::*};
use avr_device::interrupt::{self, Mutex};
use core::cell::{Cell, RefCell};
use panic_halt as _;
use turntable_core::{
    Command, Controller, Error, LineAssembler, Planner, RxBuffer, State, RATIO_X100,
};

mod console_format;
use console_format::{ConsoleWriter, Number};

// RefCell gives temporary mutable access under the interrupt mutex. On AVR an
// i32 position takes four separate byte reads; Cell/RefCell alone would not make
// those reads atomic. Every access from main *and* either ISR uses this mutex.
static MOTION: Mutex<RefCell<Controller>> = Mutex::new(RefCell::new(Controller::new()));
static RX: Mutex<RefCell<RxBuffer>> = Mutex::new(RefCell::new(RxBuffer::new()));
static DONE: Mutex<Cell<Option<i32>>> = Mutex::new(Cell::new(None));

/// Map the core's IN1..IN4 notation (bits 3..0) to the wiring PD2..PD5.
/// Preserve PD0/PD1 (USART) and PD6/PD7: writing an entire raw nibble to PORTD
/// would otherwise affect unrelated pins. This function is called only before
/// enabling interrupts or from the timer ISR, so its read/modify/write cannot
/// race another PORTD writer. No main-loop GPIO code writes these four pins.
fn write_coils(pattern: u8) {
    let bits = ((pattern & 0b1000) >> 1)
        | ((pattern & 0b0100) << 1)
        | ((pattern & 0b0010) << 3)
        | ((pattern & 0b0001) << 5);
    // SAFETY: the PAC register block contains volatile, interior-mutable
    // registers. We only touch the motor output bits under the ownership rule
    // above; the pin tokens are retained in main and never used as writers.
    let port = unsafe { &*pac::PORTD::ptr() };
    port.portd()
        .modify(|r, w| unsafe { w.bits((r.bits() & !0b0011_1100) | bits) });
}

#[avr_device::interrupt(atmega328p)]
fn USART_RX() {
    // SAFETY: the HAL transmitter owns only the TX path after split(). This
    // ISR is the sole RX reader. UDR0 has separate RX/TX hardware buffers, so
    // reading here does not consume or overwrite a byte being transmitted.
    let usart = unsafe { &*pac::USART0::ptr() };
    // DOR0 = lost hardware byte, FE0 = bad stop bit, UPE0 = bad parity. Read
    // status FIRST: reading UDR0 advances the receive FIFO and its error bits.
    // Do not use the HAL reader here: it discards this error information.
    let status = usart.ucsr0a().read();
    let byte = usart.udr0().read().bits();
    let damaged =
        status.dor0().bit_is_set() || status.fe0().bit_is_set() || status.upe0().bit_is_set();
    interrupt::free(|cs| RX.borrow(cs).borrow_mut().receive(byte, damaged));
    // Never print in an ISR. The 32-byte queue gives main time for ufmt's
    // blocking TX; overflow is sticky and rejects input instead of changing it.
}

#[avr_device::interrupt(atmega328p)]
fn TIMER2_COMPA() {
    let output = interrupt::free(|cs| {
        let output = MOTION.borrow(cs).borrow_mut().tick();
        if let Some(position) = output.done {
            // There can be only one in-flight move. main detaches an old event
            // atomically before it accepts a new move, so this cannot overwrite
            // an unreported completion. Retain the *completed* position.
            DONE.borrow(cs).set(Some(position));
        }
        output
    });
    write_coils(output.pattern);
    // Controller::tick contains only counters, i32 +/-1 and table lookup.
    // Degree/stop i64 multiplication/division is exclusively in main.
}

fn configure_timer(timer: pac::TC2) {
    // Timer2 has its own prescaler, including /128 (Timer0/1's choices differ).
    // Stop the clock while programming it, select CTC (clear on compare A),
    // and leave COM2A/COM2B disconnected: GPIO, not timer PWM, drives the coils.
    timer.tccr2b().write(|w| w.cs2().no_clock());
    timer.tccr2a().write(|w| w.wgm2().ctc());
    timer.tcnt2().write(|w| unsafe { w.bits(0) });
    // 16,000,000 / 128 = 125,000 counter increments per second.
    // The counter visits 0..=124, i.e. 125 counts, so 125,000/125 = 1,000 Hz.
    timer.ocr2a().write(|w| unsafe { w.bits(124) });
    // Timer flags clear by writing ONE. Clear any stale match before unmasking
    // OCIE2A, then start the prescaler. Global interrupts are still off here.
    timer.tifr2().write(|w| w.ocf2a().set_bit());
    timer.timsk2().write(|w| w.ocie2a().set_bit());
    timer.tccr2b().write(|w| w.cs2().prescale_128());
    // TC0 remains available for later timing; TC1 remains free for servo PWM.
}

fn print_done<W: ufmt::uWrite>(serial: &mut W, position: i32) {
    // Deliberately ignore only a transport write error. The actual HAL writer
    // is Infallible; generic helpers avoid spelling its board-specific type.
    let _ = ufmt::uwriteln!(serial, "DONE {}\r", Number::signed(position));
}

fn print_error<W: ufmt::uWrite>(serial: &mut W, error: Error) {
    let _ = ufmt::uwriteln!(serial, "ERR {}\r", error.as_str());
}

fn dispatch<W: ufmt::uWrite>(serial: &mut W, planner: &mut Planner, command: Command) {
    let snapshot = interrupt::free(|cs| MOTION.borrow(cs).borrow().snapshot());
    // This is intentionally OUTSIDE every critical section. AVR's software
    // i64 division takes many instructions; masking RX for that long can lose
    // bytes at 57600 baud (one incoming byte about every 174 microseconds).
    let prepared = match planner.prepare(command, snapshot) {
        Ok(prepared) => prepared,
        Err(error) => {
            print_error(serial, error);
            return;
        }
    };
    let (previous_done, result, snapshot) = interrupt::free(|cs| {
        // Check/apply and event detachment are one transaction. The ISR may
        // have completed between the earlier snapshot and this lock. Idle is
        // still available; a pending event must not produce an extra BUSY.
        let previous_done = DONE.borrow(cs).take();
        let mut motion = MOTION.borrow(cs).borrow_mut();
        let result = motion.apply(prepared);
        (previous_done, result, motion.snapshot())
    });
    if let Some(position) = previous_done {
        print_done(serial, position);
    }
    let output = match result {
        Ok(output) => output,
        Err(error) => {
            print_error(serial, error);
            return;
        }
    };
    // Planner is main-only. Advancing k or rebasing origin cannot race an ISR,
    // and a rejected command above never commits either change.
    planner.commit(prepared);
    match command {
        Command::Pos => {
            let _ = ufmt::uwriteln!(
                serial,
                "POS {} {}\r",
                Number::signed(snapshot.position),
                Number::unsigned(planner.k())
            );
        }
        Command::Status => {
            let _ = ufmt::uwriteln!(
                serial,
                "STATUS {} {} {} {} {}\r",
                snapshot.state.as_str(),
                Number::unsigned(u32::from(snapshot.period_ms)),
                Number::unsigned(planner.ratio_x100()),
                Number::unsigned(u32::from(planner.stops())),
                Number::unsigned(planner.k())
            );
        }
        _ => {
            let _ = ufmt::uwriteln!(serial, "OK\r");
        }
    }
    // Zero-length STEP/ROT produces its event immediately, after its own OK.
    // This value stays local so consecutive zero moves cannot overwrite it.
    if let Some(position) = output.done {
        print_done(serial, position);
    }
}

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    // The HAL sets asynchronous USART0, 8 data bits, no parity, one stop bit.
    // RXCIE0 enables RX-complete interrupts; TX stays a blocking main-only
    // ufmt writer. Retain the unused reader token so nobody polls RX as well.
    let mut serial = arduino_hal::default_serial!(dp, pins, 57600);
    serial.listen(arduino_hal::hal::usart::Event::RxComplete);
    let (_receiver, serial) = serial.split();
    let mut serial = ConsoleWriter(serial);
    let mut led = pins.d13.into_output();
    let _motor_pins = (
        pins.d2.into_output(),
        pins.d3.into_output(),
        pins.d4.into_output(),
        pins.d5.into_output(),
    );
    write_coils(0);
    configure_timer(dp.TC2);

    // The spec leaves the initial stop count open; 36 matches its closure
    // exercise. STOPS changes this at runtime and rebases the planner origin.
    let mut planner = Planner::new(RATIO_X100, 36).unwrap();
    let mut assembler = LineAssembler::new();
    // SAFETY: shared statics and peripherals are fully initialised. Neither
    // ISR enables nested interrupts, prints, or borrows main's stack objects.
    unsafe { interrupt::enable() };
    ufmt::uwriteln!(&mut serial, "turntable-firmware boot\r").unwrap_infallible();

    loop {
        let (done, item, state) = interrupt::free(|cs| {
            (
                DONE.borrow(cs).take(),
                RX.borrow(cs).borrow_mut().pop(),
                MOTION.borrow(cs).borrow().snapshot().state,
            )
        });
        if let Some(position) = done {
            print_done(&mut serial, position);
        }
        // D13 is an activity indication, including hold and settle. It has a
        // separate PORTB register, so it cannot disturb the ISR's PORTD coils.
        if state == State::Idle {
            led.set_low();
        } else {
            led.set_high();
        }
        if let Some(item) = item {
            if let Some(line) = assembler.push(item.byte, item.corrupted) {
                match line.and_then(Command::parse) {
                    Ok(command) => dispatch(&mut serial, &mut planner, command),
                    Err(error) => print_error(&mut serial, error),
                }
            }
        }
    }
}
