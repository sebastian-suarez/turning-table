//! Two interrupt-driven steppers; main owns planning, EEPROM and console TX.
//! The Nano's 328PB executes the pinned ATmega328P instruction/peripheral subset.

#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]
#![feature(asm_experimental_arch)]

use arduino_hal::{pac, prelude::*};
use avr_device::interrupt::{self, CriticalSection, Mutex};
use core::cell::{Cell, UnsafeCell};
use panic_halt as _;
use turntable_core::homing::{BUTTON_DEBOUNCE_TICKS, SWITCH_DEBOUNCE_TICKS};
use turntable_core::{
    accept, admit, button_action, Activity, ButtonAction, Command, Controller, Debouncer, Error,
    Homing, HomingStep, LineAssembler, Planner, PreparedCommand, RxBuffer, Scan, ScanMove,
    Settings, Snapshot, State, ARM_MAX_TENTHS,
};

mod console_format;
mod eeprom;
mod memory;
use console_format::{newline, signed_field, unsigned_field, ConsoleWriter};

// Every shared word is protected, even in an ISR. An AVR i32 takes four byte
// accesses; Cell alone cannot make a position/mailbox atomic against the timer.
static MOTION: Mutex<UnsafeCell<Controller>> = Mutex::new(UnsafeCell::new(Controller::new()));
static ARM: Mutex<UnsafeCell<Controller>> = Mutex::new(UnsafeCell::new(Controller::new()));
static RX: Mutex<UnsafeCell<RxBuffer>> = Mutex::new(UnsafeCell::new(RxBuffer::new()));
static DONE: Mutex<Cell<Option<i32>>> = Mutex::new(Cell::new(None));
static ADONE: Mutex<Cell<Option<i32>>> = Mutex::new(Cell::new(None));
static INPUT: Mutex<Cell<Inputs>> = Mutex::new(Cell::new(Inputs {
    home: Debouncer::new(SWITCH_DEBOUNCE_TICKS),
    button: Debouncer::new(BUTTON_DEBOUNCE_TICKS),
    presses: 0,
    armed: false,
    halted: false,
    ticks: 0,
}));

#[derive(Clone, Copy)]
struct Inputs {
    home: Debouncer,
    button: Debouncer,
    presses: u8,
    armed: bool,
    halted: bool,
    ticks: u32,
}

/// Scope each motor borrow to one interrupt lock, without a runtime counter.
fn with_motor<T>(
    motor: &Mutex<UnsafeCell<Controller>>,
    cs: CriticalSection,
    f: impl FnOnce(&mut Controller) -> T,
) -> T {
    // SAFETY: all callers hold the mutex and never re-enter this helper for
    // the same motor. The timer nests ARM then MOTION, which are distinct.
    // No borrowed reference escapes the closure, and ISRs never enable nesting.
    unsafe { f(&mut *motor.borrow(cs).get()) }
}

/// RX has one scoped access per interrupt lock, with no nested RX access.
/// Avoid a second runtime borrow counter in the latency-sensitive transaction.
fn with_rx<T>(cs: CriticalSection, f: impl FnOnce(&mut RxBuffer) -> T) -> T {
    // SAFETY: every caller holds the interrupt mutex. The closures below never
    // re-enter with_rx; their borrowed reference cannot escape in T. Neither
    // ISR enables nesting, so no other mutable reference can coexist with this.
    unsafe { f(&mut *RX.borrow(cs).get()) }
}

// Copying this small value under the mutex needs no run-time borrow counter.
// LLVM keeps only the fields a closure actually reads or changes.
fn inputs<T>(cs: CriticalSection, f: impl FnOnce(&mut Inputs) -> T) -> T {
    let mut input = INPUT.borrow(cs).get();
    let result = f(&mut input);
    INPUT.borrow(cs).set(input);
    result
}

/// Only this ISR (or boot before SEI) writes PORTD/PORTB. Preserve USART,
/// D9/OC1A and the spare pins; PB5's LED shares PORTB with arm IN3/IN4.
fn write_coils(platter: u8, arm: u8, active: bool) {
    let pd = ((platter & 8) >> 1)
        | ((platter & 4) << 1)
        | ((platter & 2) << 3)
        | ((platter & 1) << 5)
        | ((arm & 8) << 3)
        | ((arm & 4) << 5);
    let pb = ((arm & 2) >> 1) | ((arm & 1) << 2) | (u8::from(active) << 5);
    // SAFETY: volatile register access under the single-writer rule above.
    unsafe {
        (*pac::PORTD::ptr())
            .portd()
            .modify(|r, w| w.bits((r.bits() & 3) | pd));
        (*pac::PORTB::ptr())
            .portb()
            .modify(|r, w| w.bits((r.bits() & !0x25) | pb));
    }
}

fn switch_closed() -> bool {
    // SAFETY: reading PINC has no side effects. The pull-up makes closed low.
    unsafe { (*pac::PORTC::ptr()).pinc().read().bits() & 2 == 0 }
}

#[avr_device::interrupt(atmega328p)]
fn USART_RX() {
    // Status must precede UDR: reading UDR advances the hardware receive FIFO.
    // DOR marks a gap before an intact byte; FE/UPE corrupt the byte itself.
    let usart = unsafe { &*pac::USART0::ptr() };
    let status = usart.ucsr0a().read();
    let byte = usart.udr0().read().bits();
    interrupt::free(|cs| {
        with_rx(cs, |rx| {
            rx.receive(
                byte,
                status.dor0().bit_is_set(),
                status.fe0().bit_is_set() || status.upe0().bit_is_set(),
            )
        });
    });
}

#[avr_device::interrupt(atmega328p)]
fn TIMER2_COMPA() {
    interrupt::free(|cs| {
        with_motor(&ARM, cs, |arm| {
            with_motor(&MOTION, cs, |platter| {
                let pins = unsafe { (*pac::PORTC::ptr()).pinc().read().bits() };
                let mut input = INPUT.borrow(cs).get();
                input.ticks = input.ticks.wrapping_add(1);
                if input.button.sample(pins & 1 == 0) {
                    input.presses = input.presses.saturating_add(1);
                }
                input.home.sample(pins & 2 == 0);
                // Sample BEFORE ticking: the threshold tick cannot emit another step.
                // Disarming makes a held switch a single halt, without restarting hold.
                // A closure after the travel ran out (Hold/Settle) cannot home.
                if input.armed
                    && input.home.closed()
                    && matches!(arm.snapshot().state, State::Energise | State::Stepping)
                {
                    arm.halt();
                    input.armed = false;
                    input.halted = true;
                }
                INPUT.borrow(cs).set(input);
                let p = platter.tick();
                let a = arm.tick();
                if let Some(position) = p.done {
                    DONE.borrow(cs).set(Some(position));
                }
                if let Some(position) = a.done {
                    ADONE.borrow(cs).set(Some(position));
                }
                write_coils(
                    p.pattern,
                    a.pattern,
                    platter.snapshot().state != State::Idle || arm.snapshot().state != State::Idle,
                );
            })
        })
    });
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

fn print_done<W: ufmt::uWrite>(serial: &mut W, position: i32, arm: bool) {
    let _ = ufmt::uwrite!(serial, "{}DONE", if arm { "A" } else { "" });
    signed_field(serial, position);
    newline(serial);
}

fn print_error<W: ufmt::uWrite>(serial: &mut W, error: Error) {
    let _ = ufmt::uwriteln!(serial, "ERR {}\r", error.as_str());
}

fn ok<W: ufmt::uWrite>(serial: &mut W) {
    let _ = ufmt::uwriteln!(serial, "OK\r");
}

fn cfg<W: ufmt::uWrite>(serial: &mut W, s: &Settings) {
    let _ = ufmt::uwrite!(serial, "CFG");
    unsigned_field(serial, s.pratio_x100);
    unsigned_field(serial, s.aratio_x100);
    unsigned_field(serial, s.ahome_tenths as u32);
    unsigned_field(serial, u32::from(s.scan.rings));
    unsigned_field(serial, u32::from(s.scan.photos));
    unsigned_field(serial, u32::from(s.scan.interval_ms));
    for &height in &s.rings.heights[..usize::from(s.rings.len)] {
        unsigned_field(serial, u32::from(height));
    }
    newline(serial);
}

fn snapshots() -> (Snapshot, Snapshot) {
    interrupt::free(|cs| {
        (
            with_motor(&MOTION, cs, |m| m.snapshot()),
            with_motor(&ARM, cs, |m| m.snapshot()),
        )
    })
}

/// Both scan channels use the same short start transaction. All geometry is
/// already checked in main; the controller still validates its destination.
#[inline(never)]
fn scan_start(arm: bool, steps: i32, period: u8) -> Result<(), Error> {
    interrupt::free(|cs| {
        let motor = if arm { &ARM } else { &MOTION };
        with_motor(motor, cs, |m| {
            m.start_with_period(steps, period).map(|_| ())
        })
    })
}

/// Main-only state is deliberately outside the mutex. No ISR plans geometry,
/// prints, or copies a scan/settings object with interrupts masked.
struct Run {
    settings: Settings,
    platter: Planner,
    arm: Planner,
    scan: Option<Scan>,
    homing: Option<Homing>,
    homed: bool,
    stopping: bool,
    turning: bool,
    completed: u8,
}

impl Run {
    fn activity(&self, p: &Snapshot, a: &Snapshot) -> Activity {
        Activity {
            platter_idle: p.state == State::Idle,
            arm_idle: a.state == State::Idle,
            scanning: self.scan.is_some(),
            homing: self.homing.is_some(),
            homed: self.homed,
        }
    }

    fn home_step<W: ufmt::uWrite>(&mut self, serial: &mut W, step: HomingStep) {
        match step {
            HomingStep::Move {
                steps,
                period_ms,
                halt_on_switch,
            } => {
                let result = interrupt::free(|cs| {
                    let mut input = INPUT.borrow(cs).get();
                    // Search samples must be consecutive AFTER arming, not a
                    // stale debounced level retained from the backoff move.
                    input.home = Debouncer::new(SWITCH_DEBOUNCE_TICKS);
                    input.armed = halt_on_switch;
                    input.halted = false;
                    INPUT.borrow(cs).set(input);
                    with_motor(&ARM, cs, |m| m.start_with_period(steps, period_ms))
                });
                if result.is_err() {
                    self.home_step(serial, HomingStep::Failed);
                }
            }
            HomingStep::Homed(position) => {
                let _ = interrupt::free(|cs| with_motor(&ARM, cs, |m| m.set_position(position)));
                self.homed = true;
                self.homing = None;
                print_done(serial, position, true);
            }
            HomingStep::Failed => {
                interrupt::free(|cs| inputs(cs, |input| input.armed = false));
                self.homed = false;
                self.homing = None;
                print_error(serial, Error::Home);
            }
        }
    }

    /// Drain completion before admitting another move on the same motor.
    /// Homing owns its arm event (the backoff is silent); scans expose both.
    #[inline(never)]
    fn service<W: ufmt::uWrite>(&mut self, serial: &mut W) {
        let p = interrupt::free(|cs| DONE.borrow(cs).take());
        let a = interrupt::free(|cs| ADONE.borrow(cs).take());
        if let Some(position) = p {
            print_done(serial, position, false);
            if self.turning {
                self.turning = false;
                if !self.stopping {
                    self.completed += 1;
                }
            }
        }
        if let Some(position) = a {
            if let Some(homing) = &mut self.homing {
                let halted = interrupt::free(|cs| INPUT.borrow(cs).get().halted);
                let step = homing.done(switch_closed(), halted);
                self.home_step(serial, step);
            } else {
                print_done(serial, position, true);
            }
        }
        let (p, a) = snapshots();
        if p.state != State::Idle || a.state != State::Idle {
            return;
        }
        if self.scan.is_some() {
            let next = if self.stopping {
                None
            } else {
                self.scan.as_mut().and_then(Iterator::next)
            };
            let result = match next {
                Some(ScanMove::Arm(steps)) => scan_start(true, steps, a.period_ms),
                Some(ScanMove::Turn) => {
                    let period = self.scan.as_ref().unwrap().turn_period_ms();
                    // NEXT keeps fractional revolution remainders across rings.
                    let prepared = self.platter.prepare(Command::Next, p);
                    prepared.and_then(|prepared| {
                        let steps = match prepared {
                            PreparedCommand::Next { steps, .. } => steps,
                            _ => unreachable!(),
                        };
                        scan_start(false, steps, period)?;
                        self.platter.commit(prepared);
                        self.turning = true;
                        Ok(())
                    })
                }
                None => {
                    self.scan = None;
                    let _ = ufmt::uwrite!(serial, "END");
                    unsigned_field(serial, u32::from(self.completed));
                    newline(serial);
                    Ok(())
                }
            };
            if let Err(error) = result {
                print_error(serial, error);
                self.stopping = true;
            }
        }
    }
}

/// This is the acceptance boundary for main-only commands. Once accepted,
/// EEPROM/TX may take time and a later RX loss affects only later commands.
/// Motor/planner transactions still use accept() under the same RX mutex.
#[inline(never)]
fn current(generation: Option<u32>) -> bool {
    interrupt::free(|cs| {
        generation.is_none_or(|generation| {
            with_rx(cs, |rx| rx.accept_if_current(generation, || ()).is_some())
        })
    })
}

#[inline(never)]
fn start_scan<W: ufmt::uWrite>(
    serial: &mut W,
    run: &mut Run,
    arguments: Option<turntable_core::ScanSettings>,
    p: &Snapshot,
    a: &Snapshot,
    generation: Option<u32>,
) -> Result<(), Error> {
    let settings = arguments.unwrap_or(run.settings.scan);
    let scan = Scan::new(&run.settings, settings, p.period_ms, a.position);
    if !current(generation) {
        return Ok(());
    }
    let scan = scan?;
    run.settings.scan = settings;
    if arguments.is_some() {
        eeprom::save(&run.settings)?;
    }
    run.scan = Some(scan);
    run.completed = 0;
    run.turning = false;
    run.stopping = false;
    run.platter.commit(PreparedCommand::Stops {
        stops: 1,
        origin: p.position,
    });
    ok(serial);
    Ok(())
}

#[inline(never)]
fn prepare_arm(run: &Run, tenths: i16, position: i32) -> Result<PreparedCommand, Error> {
    if !(run.settings.ahome_tenths..=ARM_MAX_TENTHS).contains(&tenths) {
        return Err(Error::Range);
    }
    run.arm
        .degrees_to_halfsteps(i32::from(tenths))
        .and_then(|target| target.checked_sub(position))
        .ok_or(Error::Range)
        .map(PreparedCommand::Move)
}

#[inline(never)]
fn platter_command<W: ufmt::uWrite>(
    serial: &mut W,
    run: &mut Run,
    prepared: Result<PreparedCommand, Error>,
    query_position: bool,
    generation: u32,
    arm: bool,
) -> Result<(), Error> {
    // A completion may have arrived during preparation. Detach it atomically
    // with acceptance so the new move cannot overwrite its predecessor's event.
    // Resolve the channel before masking. black_box prevents LTO from
    // expanding every pointer access back into two static-channel branches;
    // those repeated branches otherwise exceed the 20 us transaction budget.
    let selected = if arm {
        (&ARM, &ADONE, &mut run.arm)
    } else {
        (&MOTION, &DONE, &mut run.platter)
    };
    let (motion, mailbox, planner) = core::hint::black_box(selected);
    let (previous, accepted) = interrupt::free(|cs| {
        let previous = mailbox.borrow(cs).take();
        let accepted = with_rx(cs, |rx| {
            with_motor(motion, cs, |m| {
                accept(rx, || &mut *m, planner, generation, prepared)
            })
        });
        (previous, accepted)
    });
    if let Some(position) = previous {
        print_done(serial, position, arm);
    }
    if let Some((result, _)) = accepted {
        let output = result?;
        if query_position {
            let _ = ufmt::uwrite!(serial, "POS");
            let position = interrupt::free(|cs| with_motor(&MOTION, cs, |m| m.snapshot()).position);
            signed_field(serial, position);
            unsigned_field(serial, run.platter.k());
            newline(serial);
        } else {
            ok(serial);
        }
        if let Some(position) = output.done {
            print_done(serial, position, arm);
        }
    }
    Ok(())
}

#[inline(never)]
fn status<W: ufmt::uWrite>(serial: &mut W, run: &Run, p: &Snapshot, a: &Snapshot) {
    let _ = ufmt::uwrite!(serial, "STATUS {}", p.state.as_str());
    unsigned_field(serial, u32::from(p.period_ms));
    unsigned_field(serial, run.platter.ratio_x100());
    unsigned_field(serial, u32::from(run.platter.stops()));
    unsigned_field(serial, run.platter.k());
    let _ = ufmt::uwrite!(serial, " {}", a.state.as_str());
    signed_field(serial, a.position);
    unsigned_field(serial, u32::from(run.homed));
    if run.scan.is_some() {
        unsigned_field(serial, u32::from(run.completed + 1));
    } else {
        let _ = ufmt::uwrite!(serial, " -");
    }
    newline(serial);
}

#[inline(never)]
fn execute<W: ufmt::uWrite>(
    serial: &mut W,
    run: &mut Run,
    command: Command,
    p: &Snapshot,
    a: &Snapshot,
    generation: Option<u32>,
) -> Result<(), Error> {
    match command {
        Command::Scan(arguments) => return start_scan(serial, run, arguments, p, a, generation),
        Command::Arm(tenths) => {
            let prepared = prepare_arm(run, tenths, a.position);
            return platter_command(serial, run, prepared, false, generation.unwrap(), true);
        }
        Command::Step(_)
        | Command::Rot(_)
        | Command::Stops(_)
        | Command::Next
        | Command::Zero
        | Command::Pos => {
            let prepared = run.platter.prepare(command, *p);
            return platter_command(
                serial,
                run,
                prepared,
                command == Command::Pos,
                generation.unwrap(),
                false,
            );
        }
        Command::Home => {
            let (homing, step) = Homing::start(switch_closed(), &run.settings);
            if !current(generation) {
                return Ok(());
            }
            run.homed = false;
            run.homing = Some(homing);
            ok(serial);
            run.home_step(serial, step);
            return Ok(());
        }
        _ => {}
    }
    if !current(generation) {
        return Ok(());
    }
    match command {
        Command::Stop => {
            // A completed turn counts even if its mailbox arrived while main
            // parsed STOP. A genuinely halted turn must never increment END.
            let idle = interrupt::free(|cs| {
                let idle = with_motor(&MOTION, cs, |m| {
                    let idle = m.snapshot().state == State::Idle;
                    m.halt();
                    idle
                });
                with_motor(&ARM, cs, Controller::halt);
                inputs(cs, |input| input.armed = false);
                idle
            });
            if run.turning && idle {
                run.completed += 1;
                run.turning = false;
            }
            if let Some(homing) = &mut run.homing {
                homing.stop();
            }
            run.stopping = true;
        }
        Command::Speed(period) => {
            interrupt::free(|cs| {
                with_motor(&MOTION, cs, |m| m.apply(PreparedCommand::Speed(period)))?;
                with_motor(&ARM, cs, |m| m.apply(PreparedCommand::Speed(period)))?;
                Ok(())
            })?;
        }
        Command::Cfg => {
            cfg(serial, &run.settings);
            return Ok(());
        }
        Command::Status => {
            status(serial, run, p, a);
            return Ok(());
        }
        _ => {
            match command {
                Command::Rings(rings) => run.settings.rings = rings,
                Command::PRatio(ratio) => {
                    run.settings.pratio_x100 = ratio;
                    run.platter.set_ratio(ratio);
                }
                Command::ARatio(ratio) => {
                    run.settings.aratio_x100 = ratio;
                    run.arm.set_ratio(ratio);
                    run.homed = false;
                }
                Command::AHome(home) => {
                    run.settings.ahome_tenths = home;
                    run.homed = false;
                }
                _ => return Err(Error::Syntax),
            }
            eeprom::save(&run.settings)?;
        }
    }
    ok(serial);
    Ok(())
}

#[inline(never)]
fn dispatch<W: ufmt::uWrite>(
    serial: &mut W,
    run: &mut Run,
    command: Result<Command, Error>,
    generation: Option<u32>,
) {
    let (p, a) = snapshots();
    let command = command.and_then(|c| admit(&c, run.activity(&p, &a)).map(|_| c));
    let result = match command {
        Ok(command) => execute(serial, run, command, &p, &a, generation),
        Err(error) if current(generation) => Err(error),
        Err(_) => Ok(()),
    };
    if let Err(error) = result {
        print_error(serial, error);
    }
}

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let mut serial = arduino_hal::default_serial!(dp, pins, 57600);
    serial.listen(arduino_hal::hal::usart::Event::RxComplete);
    let (_receiver, serial) = serial.split();
    let mut serial = ConsoleWriter(serial);
    let _outputs = (
        pins.d2.into_output(),
        pins.d3.into_output(),
        pins.d4.into_output(),
        pins.d5.into_output(),
        pins.d6.into_output(),
        pins.d7.into_output(),
        pins.d8.into_output(),
        pins.d10.into_output(),
        pins.d13.into_output(),
    );
    let _inputs = (pins.a0.into_pull_up_input(), pins.a1.into_pull_up_input());
    write_coils(0, 0, false);
    let (settings, defaults) = eeprom::boot();
    // Keep the long-lived runner out of main's AVR stack frame. Accessing
    // stack offsets above 63 needs extra address/SREG instructions on AVR;
    // static storage also makes this persistent RAM visible to avr-size.
    static mut RUN: core::mem::MaybeUninit<Run> = core::mem::MaybeUninit::uninit();
    // SAFETY: entry runs once. This is the sole reference, retained by main;
    // neither ISR can name/access RUN. Initialize it before enabling interrupts.
    let run = unsafe {
        (*core::ptr::addr_of_mut!(RUN)).write(Run {
            platter: Planner::new(settings.pratio_x100, 36).unwrap(),
            arm: Planner::new(settings.aratio_x100, 1).unwrap(),
            settings,
            scan: None,
            homing: None,
            homed: false,
            stopping: false,
            turning: false,
            completed: 0,
        })
    };
    let mut assembler = LineAssembler::new();
    configure_timer(dp.TC2);
    // SAFETY: peripherals/statics are initialized; neither ISR enables nesting.
    unsafe { interrupt::enable() };
    ufmt::uwriteln!(&mut serial, "turntable-firmware boot\r").unwrap_infallible();
    if defaults {
        let _ = ufmt::uwriteln!(&mut serial, "DEFAULTS\r");
    }
    cfg(&mut serial, &run.settings);
    loop {
        run.service(&mut serial);
        let press = interrupt::free(|cs| {
            inputs(cs, |input| {
                if input.presses == 0 {
                    false
                } else {
                    input.presses -= 1;
                    true
                }
            })
        });
        if press {
            let (p, a) = snapshots();
            let command = match button_action(run.activity(&p, &a)) {
                ButtonAction::Stop => Command::Stop,
                ButtonAction::Home => Command::Home,
                ButtonAction::Scan => Command::Scan(None),
            };
            dispatch(&mut serial, run, Ok(command), None);
        }
        let item = interrupt::free(|cs| with_rx(cs, RxBuffer::pop));
        if let Some(item) = item {
            if let Some(line) = assembler.push(item.byte, item.corrupted) {
                if item.corrupted {
                    print_error(&mut serial, Error::Line);
                } else {
                    dispatch(
                        &mut serial,
                        run,
                        line.and_then(Command::parse),
                        Some(item.generation),
                    );
                }
            }
        }
    }
}
