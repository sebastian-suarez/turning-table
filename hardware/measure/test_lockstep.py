"""Host checks for lockstep.py's resend rule, using a scripted port instead of serial hardware.

Run from the repository root:
  python3 -I -m unittest discover -s hardware/measure -p 'test_*.py'
"""
import contextlib
import io
import unittest

import lockstep


class FakePort:
    """Replays the board's lines in order and records every line the script sends."""

    def __init__(self, *lines):
        self.lines = list(lines)
        self.sent = []

    def send(self, line):
        self.sent.append(line)

    def read_line(self, timeout):
        return self.lines.pop(0) if self.lines else None  # None is what the real port returns on a timeout

    def stamp(self):
        return "   0.000s"


class RunCommandTest(unittest.TestCase):
    def run_command(self, port, cmd):
        with contextlib.redirect_stdout(io.StringIO()):  # keep the script's transcript out of the test output
            return lockstep.run_command(port, cmd, move_timeout=1)

    def test_err_line_after_ok_does_not_resend_the_accepted_move(self):
        port = FakePort("OK", "ERR LINE", "DONE 10")
        self.assertEqual(self.run_command(port, "STEP 10"), "DONE 10")
        self.assertEqual(port.sent, ["STEP 10"])

    def test_err_line_as_first_reply_resends_once(self):
        port = FakePort("ERR LINE", "OK", "DONE 10")
        self.assertEqual(self.run_command(port, "STEP 10"), "DONE 10")
        self.assertEqual(port.sent, ["STEP 10", "STEP 10"])


if __name__ == "__main__":
    unittest.main()
