"""Guarded native XTest input for desktop probes, without changing key maps.

Text is limited to the unshifted ASCII workload. Each stroke checks focus, held
keys, XKB modifiers and the active layout while briefly holding XGrabServer.
No sleep or application wait occurs under the server grab. XSendEvent is not
used: it did not reliably deliver the workload to the native editor.
"""

import ctypes as C
import time


class KeyboardState(C.Structure):
    _fields_ = [
        ("group", C.c_ubyte),
        ("locked_group", C.c_ubyte),
        ("base_group", C.c_ushort),
        ("latched_group", C.c_ushort),
        ("mods", C.c_ubyte),
        ("base_mods", C.c_ubyte),
        ("latched_mods", C.c_ubyte),
        ("locked_mods", C.c_ubyte),
        ("compat_state", C.c_ubyte),
        ("grab_mods", C.c_ubyte),
        ("compat_grab_mods", C.c_ubyte),
        ("lookup_mods", C.c_ubyte),
        ("compat_lookup_mods", C.c_ubyte),
        ("ptr_buttons", C.c_ushort),
    ]


def unshifted_ascii(value):
    if any(not (" " <= char <= "~") or char.isupper() for char in value):
        raise ValueError(
            "Native typing probe only supports unshifted ASCII; keyboard mapping is never changed"
        )


def validate_keyboard_state(state, num_lock, keys):
    if any(keys):
        raise AssertionError("A keyboard key is held; refusing native input")
    if (
        state.base_mods
        or state.latched_mods
        or state.mods & ~num_lock
        or state.ptr_buttons
    ):
        raise AssertionError("Unsafe modifier/button state; refusing native input")


def chord_symbols(command):
    parts = command.split("+")
    modifiers = {"ctrl": 0xFFE3, "shift": 0xFFE1, "alt": 0xFFE9}
    named = {
        "Home": 0xFF50,
        "End": 0xFF57,
        "Tab": 0xFF09,
        "Left": 0xFF51,
        "Right": 0xFF53,
        "Escape": 0xFF1B,
        "Return": 0xFF0D,
        "BackSpace": 0xFF08,
    }
    if (
        any(part not in modifiers for part in parts[:-1])
        or len(set(parts[:-1])) != len(parts) - 1
    ):
        raise ValueError(f"Unsupported native input chord: {command}")
    key = parts[-1]
    if key in named:
        symbol = named[key]
    elif len(key) == 1 and ("a" <= key <= "z" or "0" <= key <= "9"):
        symbol = ord(key)
    else:
        raise ValueError(f"Unsupported native input key: {key}")
    return [modifiers[part] for part in parts[:-1]], symbol


class NativeInput:
    def __init__(self, display_name):
        self.x = C.CDLL("libX11.so.6")
        self.t = C.CDLL("libXtst.so.6")
        D = C.c_void_p

        def api(lib, name, args, result=C.c_int):
            function = getattr(lib, name)
            function.argtypes = args
            function.restype = result
            return function

        self.open = api(self.x, "XOpenDisplay", [C.c_char_p], D)
        self.close = api(self.x, "XCloseDisplay", [D])
        self.grab = api(self.x, "XGrabServer", [D])
        self.ungrab = api(self.x, "XUngrabServer", [D])
        self.sync = api(self.x, "XSync", [D, C.c_int])
        self.focus = api(
            self.x, "XGetInputFocus", [D, C.POINTER(C.c_ulong), C.POINTER(C.c_int)]
        )
        self.code = api(self.x, "XKeysymToKeycode", [D, C.c_ulong], C.c_ubyte)
        self.fake_key = api(
            self.t, "XTestFakeKeyEvent", [D, C.c_uint, C.c_int, C.c_ulong]
        )
        self.fake_button = api(
            self.t, "XTestFakeButtonEvent", [D, C.c_uint, C.c_int, C.c_ulong]
        )
        self.keymap = api(self.x, "XQueryKeymap", [D, C.POINTER(C.c_ubyte)])
        self.state = api(self.x, "XkbGetState", [D, C.c_uint, C.POINTER(KeyboardState)])
        self.lookup = api(
            self.x,
            "XkbLookupKeySym",
            [D, C.c_ubyte, C.c_uint, C.POINTER(C.c_uint), C.POINTER(C.c_ulong)],
        )
        self.modifiers = api(self.x, "XkbKeysymToModifiers", [D, C.c_ulong], C.c_uint)
        self.root = api(self.x, "XDefaultRootWindow", [D], C.c_ulong)
        self.pointer = api(
            self.x,
            "XQueryPointer",
            [
                D,
                C.c_ulong,
                C.POINTER(C.c_ulong),
                C.POINTER(C.c_ulong),
                C.POINTER(C.c_int),
                C.POINTER(C.c_int),
                C.POINTER(C.c_int),
                C.POINTER(C.c_int),
                C.POINTER(C.c_uint),
            ],
        )
        query = api(self.t, "XTestQueryExtension", [D, *([C.POINTER(C.c_int)] * 4)])
        self.display = self.open(display_name.encode())
        if not self.display:
            raise RuntimeError("XOpenDisplay failed for native input observer")
        event, error, major, minor = (C.c_int() for _ in range(4))
        if not query(
            self.display, C.byref(event), C.byref(error), C.byref(major), C.byref(minor)
        ):
            self.close(self.display)
            self.display = None
            raise RuntimeError("XTest extension unavailable")
        self.version = (major.value, minor.value)

    def __enter__(self):
        return self

    def __exit__(self, *_):
        if self.display:
            self.close(self.display)
            self.display = None

    def guard(self, window):
        target, revert = C.c_ulong(), C.c_int()
        self.focus(self.display, C.byref(target), C.byref(revert))
        if target.value != int(window):
            raise AssertionError("Focus lost; refusing native input")
        keys = (C.c_ubyte * 32)()
        self.keymap(self.display, keys)
        state = KeyboardState()
        if self.state(self.display, 0x0100, C.byref(state)) != 0:
            raise RuntimeError("XkbGetState failed")
        num_lock = self.modifiers(self.display, 0xFF7F)
        validate_keyboard_state(state, num_lock, keys)
        return state

    def pointer_over(self, window):
        current = self.root(self.display)
        # Check ancestry, not just coordinates: another window can obscure the target.
        for _ in range(32):
            if current == int(window):
                return True
            root, child = C.c_ulong(), C.c_ulong()
            rx, ry, wx, wy = (C.c_int() for _ in range(4))
            mask = C.c_uint()
            if (
                not self.pointer(
                    self.display,
                    current,
                    C.byref(root),
                    C.byref(child),
                    C.byref(rx),
                    C.byref(ry),
                    C.byref(wx),
                    C.byref(wy),
                    C.byref(mask),
                )
                or not child.value
            ):
                return False
            current = child.value
        return False

    def press_and_release(self, fake, details, evidence=None):
        """Release only keys/buttons this call pressed, before releasing the server."""
        owned = []
        released = []
        if evidence is not None:
            evidence["pressed_codes"] = owned
            evidence["released_codes"] = released
        try:
            for detail in details:
                if not fake(self.display, detail, 1, 0):
                    raise RuntimeError("XTest press failed")
                owned.append(detail)
        finally:
            failures = []
            for detail in reversed(owned):
                try:
                    if not fake(self.display, detail, 0, 0):
                        raise RuntimeError("XTest release failed")
                    released.append(detail)
                except Exception as error:
                    failures.append((detail, repr(error)))
            self.sync(self.display, 0)
            if evidence is not None:
                evidence["release_failures"] = failures
            if failures:
                raise RuntimeError(f"Could not release probe-owned input: {failures}")

    def checked_keycode(self, symbol, state):
        code = self.code(self.display, symbol)
        consumed, actual = C.c_uint(), C.c_ulong()
        modifiers = state.mods | ((state.group & 3) << 13)
        if (
            not code
            or not self.lookup(
                self.display, code, modifiers, C.byref(consumed), C.byref(actual)
            )
            or actual.value != symbol
        ):
            raise AssertionError(
                "Current layout does not produce requested key; refusing native input"
            )
        return code

    def chord(self, window, command, evidence=None):
        modifier_symbols, symbol = chord_symbols(command)
        result = evidence if evidence is not None else {}
        result.update({"backend": "XTest", "command": command, "completed": False})
        started = time.monotonic()
        self.grab(self.display)
        try:
            state = self.guard(window)
            # Resolve everything before pressing a modifier or key.
            codes = [
                self.checked_keycode(value, state)
                for value in (*modifier_symbols, symbol)
            ]
            if len(set(codes)) != len(codes):
                raise AssertionError("Chord resolves to duplicate physical keys")
            self.press_and_release(self.fake_key, codes, result)
            result["completed"] = True
        finally:
            self.ungrab(self.display)
            self.sync(self.display, 0)
            result["elapsed_seconds"] = time.monotonic() - started
        return result

    def stroke(self, window, *, char=None, button=None):
        self.grab(self.display)
        try:
            state = self.guard(window)
            if char is not None:
                code = self.checked_keycode(ord(char), state)
                fake, detail = self.fake_key, code
            else:
                if button not in (4, 5) or not self.pointer_over(window):
                    raise AssertionError(
                        "Pointer is not over the focused test window; refusing wheel input"
                    )
                fake, detail = self.fake_button, button
            self.press_and_release(fake, [detail])
        finally:
            self.ungrab(self.display)
            self.sync(self.display, 0)

    def send(
        self, window, *, text=None, button=None, count=1, delay=0.01, evidence=None
    ):
        if text is not None:
            unshifted_ascii(text)
            items = list(text)
        else:
            items = [button] * count
        if delay < 0:
            raise ValueError("Input delay cannot be negative")
        result = evidence if evidence is not None else {}
        result.update(
            {
                "backend": "XTest",
                "xtest_version": self.version,
                "requested_inter_event_delay_seconds": delay,
                "requested_events": len(items),
                "delivered_events": 0,
                "event_timings": [],
                "limits": "Timings include server synchronization and observer scheduling; they are not editor response or presentation latency.",
            }
        )
        started = time.monotonic()
        try:
            for index, item in enumerate(items):
                at = time.monotonic()
                self.stroke(
                    window,
                    char=item if text is not None else None,
                    button=item if text is None else None,
                )
                result["event_timings"].append(
                    {
                        "index": index,
                        "start_seconds": at - started,
                        "duration_seconds": time.monotonic() - at,
                    }
                )
                result["delivered_events"] += 1
                if index + 1 < len(items):
                    time.sleep(delay)
        finally:
            elapsed = time.monotonic() - started
            result["elapsed_seconds"] = elapsed
            result["actual_events_per_second"] = (
                result["delivered_events"] / elapsed if elapsed else 0
            )
        return result
