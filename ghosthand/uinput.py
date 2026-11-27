"""A virtual absolute pointer made through /dev/uinput.

The compositor sees a real mouse: hover, press, release, drag and the wheel all work, on layer-shell surfaces (bars, panels, launchers)
as well as windows. Needs write access to /dev/uinput (see docs/setup.md)."""
import fcntl
import json
import os
import struct
import time

from .paths import last_position_file, runtime_dir

BTN_LEFT, BTN_RIGHT, BTN_MIDDLE = 0x110, 0x111, 0x112
EV_SYN, EV_KEY, EV_REL, EV_ABS = 0, 1, 2, 3
REL_HWHEEL, REL_WHEEL, REL_WHEEL_HI_RES, REL_HWHEEL_HI_RES = 6, 8, 11, 12
ABS_X, ABS_Y = 0, 1
UI_SET_EVBIT, UI_SET_KEYBIT, UI_SET_RELBIT, UI_SET_ABSBIT = 0x40045564, 0x40045565, 0x40045566, 0x40045567
UI_DEV_CREATE, UI_DEV_DESTROY = 0x5501, 0x5502
LAST_MAX_AGE = 600  # a remembered position older than this is not trusted: the real pointer has moved since


def read_last(path=None, now=time.time):
    """Where the last run left the pointer, if that is recent enough to trust: (x, y) or None."""
    try:
        with open(path or last_position_file()) as f:
            d = json.load(f)
        if now() - d["t"] <= LAST_MAX_AGE:
            return float(d["x"]), float(d["y"])
    except (OSError, ValueError, KeyError, TypeError):
        pass
    return None


class Pointer:
    """Coordinates are screenshot pixels of the output. `announce(x, y)` gets logical px after every step (for the cursor tag)."""

    def __init__(self, info, announce=None):
        self.w, self.h, self.scale, self.output = info["width"], info["height"], info["scale"], info["name"]
        self.known = False
        self.x = self.y = 0.0
        last = read_last()
        if last:
            self.x, self.y = last
            self.known = True
        self.announce = announce
        self.down = set()
        self.fd = os.open("/dev/uinput", os.O_WRONLY | os.O_NONBLOCK)
        for ev in (EV_KEY, EV_REL, EV_ABS, EV_SYN):
            fcntl.ioctl(self.fd, UI_SET_EVBIT, ev)
        for key in (BTN_LEFT, BTN_RIGHT, BTN_MIDDLE):
            fcntl.ioctl(self.fd, UI_SET_KEYBIT, key)
        for rel in (REL_WHEEL, REL_HWHEEL, REL_WHEEL_HI_RES, REL_HWHEEL_HI_RES):
            fcntl.ioctl(self.fd, UI_SET_RELBIT, rel)
        for axis in (ABS_X, ABS_Y):
            fcntl.ioctl(self.fd, UI_SET_ABSBIT, axis)
        absmax = [0] * 64
        absmax[ABS_X] = absmax[ABS_Y] = 32767
        head = struct.pack("80sHHHHi", b"ghosthand-pointer", 3, 0x1234, 0x5678, 1, 0)
        os.write(self.fd, head + struct.pack("64i", *absmax) + struct.pack("64i", *[0] * 64) * 3)
        fcntl.ioctl(self.fd, UI_DEV_CREATE)
        time.sleep(1.0)  # let the compositor pick the device up

    def close(self):
        for code in list(self.down):  # never leave a button held
            self.button(code, False)
        time.sleep(0.2)
        fcntl.ioctl(self.fd, UI_DEV_DESTROY)
        os.close(self.fd)
        try:
            runtime_dir().mkdir(parents=True, exist_ok=True)
            with open(last_position_file(), "w") as f:
                json.dump({"x": self.x, "y": self.y, "t": time.time()}, f)
        except OSError:
            pass

    def emit(self, kind, code, value):
        os.write(self.fd, struct.pack("llHHi", 0, 0, kind, code, value))

    def sync(self):
        self.emit(EV_SYN, 0, 0)

    def put(self, x, y):
        self.x, self.y, self.known = x, y, True
        self.emit(EV_ABS, ABS_X, int(max(0, min(1, x / self.w)) * 32767))
        self.emit(EV_ABS, ABS_Y, int(max(0, min(1, y / self.h)) * 32767))
        self.sync()
        if self.announce:
            self.announce(x / self.scale, y / self.scale)

    def glide(self, x, y, steps=14):
        if not self.known:  # no idea where the pointer is: gliding from a corner would sweep across the screen
            self.put(x, y)
            return
        x0, y0 = self.x, self.y
        for i in range(1, steps + 1):
            self.put(x0 + (x - x0) * i / steps, y0 + (y - y0) * i / steps)
            time.sleep(0.012)

    def button(self, code, is_down):
        (self.down.add if is_down else self.down.discard)(code)
        self.emit(EV_KEY, code, 1 if is_down else 0)
        self.sync()

    def click(self, code, times=1):
        for i in range(times):
            if i:
                time.sleep(0.08)
            self.button(code, True)
            time.sleep(0.12)
            self.button(code, False)

    def wheel(self, notches, horizontal=False):
        """Positive scrolls the content down (or right). One notch is 120 hi-res units."""
        step = 1 if notches > 0 else -1
        for _ in range(abs(int(round(notches)))):
            self.emit(EV_REL, REL_HWHEEL_HI_RES if horizontal else REL_WHEEL_HI_RES, 120 * step if horizontal else -120 * step)
            self.emit(EV_REL, REL_HWHEEL if horizontal else REL_WHEEL, step if horizontal else -step)
            self.sync()
            time.sleep(0.05)
