#!/usr/bin/env python3
"""Drive the turntable console over USB serial, one command at a time (lockstep).

Standard library only (termios), so it runs on the Mac without installing anything.
Close the `cargo run` console first: the port is exclusive. Opening the port resets the board
(the USB-serial chip pulls the reset line), so the script waits for the boot banner before talking.

Examples
  lockstep.py /dev/cu.usbserial-A5069RR4 STATUS POS
  lockstep.py /dev/cu.usbserial-A5069RR4 "SPEED 10" ZERO "STEP 40760"      # one calibration run
  lockstep.py /dev/cu.usbserial-A5069RR4 ZERO "STOPS 36" --repeat 360 NEXT # 10 revolutions of 36 stops
  lockstep.py /dev/cu.usbserial-A5069RR4 --spam-status "ROT 3600"          # time a 10-turn move under STATUS traffic
  lockstep.py /dev/cu.usbserial-A5069RR4 --listen 5                        # just print what the board says
"""
import argparse
import os
import select
import sys
import termios
import time

BAUDS = {9600: termios.B9600, 57600: termios.B57600, 115200: termios.B115200}
MOTION = ("STEP", "ROT", "NEXT")  # commands answered by OK and later by DONE


class Port:
    def __init__(self, path, baud):
        self.fd = os.open(path, os.O_RDWR | os.O_NOCTTY | os.O_NONBLOCK)
        iflag, oflag, cflag, lflag, _, _, cc = termios.tcgetattr(self.fd)
        iflag, oflag, lflag = 0, 0, 0                      # raw: no CR/LF translation, no echo
        cflag = termios.CS8 | termios.CREAD | termios.CLOCAL  # 8N1, ignore modem lines
        cc[termios.VMIN], cc[termios.VTIME] = 0, 0
        termios.tcsetattr(self.fd, termios.TCSANOW, [iflag, oflag, cflag, lflag, BAUDS[baud], BAUDS[baud], cc])
        self.buf = b""
        self.t0 = time.monotonic()

    def stamp(self):
        return f"{time.monotonic() - self.t0:8.3f}s"

    def send(self, line):
        os.write(self.fd, line.encode("ascii") + b"\n")
        print(f"{self.stamp()}  > {line}")

    def read_line(self, timeout):
        """Return the next complete line (without CR/LF) or None on timeout."""
        deadline = time.monotonic() + timeout
        while True:
            if b"\n" in self.buf:
                raw, self.buf = self.buf.split(b"\n", 1)
                return raw.replace(b"\r", b"").decode("ascii", "replace")
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                return None
            ready, _, _ = select.select([self.fd], [], [], min(remaining, 0.5))
            if ready:
                try:
                    chunk = os.read(self.fd, 256)
                except BlockingIOError:
                    chunk = b""
                if chunk:
                    self.buf += chunk

    def drain(self, seconds, tag="  < "):
        end = time.monotonic() + seconds
        while True:
            line = self.read_line(max(0.0, end - time.monotonic()))
            if line is None:
                return
            print(f"{self.stamp()}{tag}{line}")


def wait_for(port, prefixes, timeout, label):
    """Print lines until one starts with any prefix; return it (or None on timeout)."""
    while True:
        line = port.read_line(timeout)
        if line is None:
            print(f"{port.stamp()}  !! timeout waiting for {label}")
            return None
        print(f"{port.stamp()}  < {line}")
        if any(line.startswith(p) for p in prefixes):
            return line


def run_command(port, cmd, move_timeout, retried=False):
    """Send one command and wait for its reply (and DONE after a move).

    Spec v3.3 host rule: ERR LINE (solicited or not) or a reply timeout right after one means the line was
    consumed as the loss-recovery marker; resend the command once. The rule covers the first reply only.
    After OK the board has accepted the move, so a resend could run it twice: the DONE wait matches DONE
    alone (other lines are still printed). An ERR LINE during that wait concerns later input: the board
    discards through the next newline, so the next command gets no reply, times out and is resent once.
    """
    verb = cmd.split()[0].upper()
    port.send(cmd)
    if verb in ("POS", "STATUS"):
        reply = wait_for(port, (verb, "ERR"), 5, verb)
    else:
        reply = wait_for(port, ("OK", "ERR"), 5, "OK/ERR")
    if reply in (None, "ERR LINE") and not retried:
        print(f"{port.stamp()}  .. line lost on the wire, resending {cmd!r} once")
        return run_command(port, cmd, move_timeout, retried=True)
    if reply == "OK" and verb in MOTION:
        reply = wait_for(port, ("DONE",), move_timeout, "DONE")
    return reply


def spam_status(port, cmd, interval, move_timeout):
    port.send(cmd)
    if wait_for(port, ("OK", "ERR"), 5, "OK/ERR") != "OK":
        return
    started = time.monotonic()
    sent = replies = errors = 0
    next_send = started
    while True:
        now = time.monotonic()
        if now >= next_send:
            os.write(port.fd, b"STATUS\n")
            sent += 1
            next_send = now + interval
        line = port.read_line(min(interval, max(0.0, next_send - time.monotonic())) or 0.01)
        if line is None:
            if time.monotonic() - started > move_timeout:
                print(f"{port.stamp()}  !! no DONE after {move_timeout}s")
                return
            continue
        if line.startswith("STATUS"):
            replies += 1
        elif line.startswith("ERR"):
            errors += 1
            print(f"{port.stamp()}  < {line}")
        elif line.startswith("DONE"):
            elapsed = time.monotonic() - started
            print(f"{port.stamp()}  < {line}")
            print(f"move took {elapsed:.2f} s from OK to DONE; STATUS sent {sent}, answered {replies}, ERR lines {errors}")
            return
        else:
            print(f"{port.stamp()}  < {line}")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("port")
    ap.add_argument("commands", nargs="*", help='commands such as STATUS, "STEP 512", "ROT -90", NEXT')
    ap.add_argument("--baud", type=int, default=57600, choices=sorted(BAUDS))
    ap.add_argument("--repeat", type=int, default=1, help="repeat the LAST command N times (earlier commands run once)")
    ap.add_argument("--move-timeout", type=float, default=900.0, help="seconds to wait for DONE")
    ap.add_argument("--spam-status", action="store_true", help="send STATUS every --interval while the first command moves")
    ap.add_argument("--interval", type=float, default=0.2)
    ap.add_argument("--listen", type=float, default=0.0, help="only print what the board sends for N seconds")
    ap.add_argument("--no-boot-wait", action="store_true", help="do not wait 2 s for the reset banner")
    args = ap.parse_intermixed_args()  # options may follow the command list, e.g. ZERO "STOPS 36" --repeat 360 NEXT

    port = Port(args.port, args.baud)
    if not args.no_boot_wait:
        port.drain(2.0, tag="  < [boot] ")
    if args.listen:
        port.drain(args.listen)
        return
    if args.spam_status:
        if not args.commands:
            sys.exit("give the move command, e.g. --spam-status \"ROT 3600\"")
        spam_status(port, args.commands[0], args.interval, args.move_timeout)
        return
    if not args.commands:
        sys.exit("no commands given (or use --listen)")
    for cmd in args.commands[:-1]:
        if run_command(port, cmd, args.move_timeout) is None:
            sys.exit(1)
    last = args.commands[-1]
    for i in range(args.repeat):
        if args.repeat > 1:
            print(f"--- {last} {i + 1}/{args.repeat}")
        if run_command(port, last, args.move_timeout) is None:
            sys.exit(1)


if __name__ == "__main__":
    main()
