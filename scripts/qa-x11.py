"""X11 input helpers for testing the Tauri desktop on Linux."""
import ctypes as C
import ctypes.util
import time

class ClientMessage(C.Structure):
    _fields_ = [('type', C.c_int), ('serial', C.c_ulong), ('send_event', C.c_int),
                ('display', C.c_void_p), ('window', C.c_ulong),
                ('message_type', C.c_ulong), ('format', C.c_int),
                ('data', C.c_long * 5)]


class XEvent(C.Union):
    _fields_ = [('client', ClientMessage), ('pad', C.c_long * 24)]


class X11:
    def __init__(self, display):
        self.x = C.CDLL(ctypes.util.find_library('X11'))
        self.t = C.CDLL(ctypes.util.find_library('Xtst'))
        signatures = {
            'XOpenDisplay': (C.c_void_p, [C.c_char_p]),
            'XDefaultRootWindow': (C.c_ulong, [C.c_void_p]),
            'XInternAtom': (C.c_ulong, [C.c_void_p, C.c_char_p, C.c_int]),
            'XGetWindowProperty': (C.c_int, [C.c_void_p, C.c_ulong, C.c_ulong,
                C.c_long, C.c_long, C.c_int, C.c_ulong, C.POINTER(C.c_ulong),
                C.POINTER(C.c_int), C.POINTER(C.c_ulong), C.POINTER(C.c_ulong),
                C.POINTER(C.POINTER(C.c_ubyte))]),
            'XFree': (C.c_int, [C.c_void_p]),
            'XStringToKeysym': (C.c_ulong, [C.c_char_p]),
            'XKeysymToKeycode': (C.c_uint, [C.c_void_p, C.c_ulong]),
            'XFlush': (C.c_int, [C.c_void_p]),
            'XSetInputFocus': (C.c_int, [C.c_void_p, C.c_ulong, C.c_int, C.c_ulong]),
            'XRaiseWindow': (C.c_int, [C.c_void_p, C.c_ulong]),
            'XSendEvent': (C.c_int, [C.c_void_p, C.c_ulong, C.c_int, C.c_long, C.POINTER(XEvent)]),
            'XGetGeometry': (C.c_int, [C.c_void_p, C.c_ulong, C.POINTER(C.c_ulong),
                C.POINTER(C.c_int), C.POINTER(C.c_int), C.POINTER(C.c_uint),
                C.POINTER(C.c_uint), C.POINTER(C.c_uint), C.POINTER(C.c_uint)]),
            'XTranslateCoordinates': (C.c_int, [C.c_void_p, C.c_ulong, C.c_ulong,
                C.c_int, C.c_int, C.POINTER(C.c_int), C.POINTER(C.c_int), C.POINTER(C.c_ulong)]),
            'XCloseDisplay': (C.c_int, [C.c_void_p]),
        }
        for name, (result, args) in signatures.items():
            fn = getattr(self.x, name)
            fn.restype, fn.argtypes = result, args
        self.t.XTestFakeKeyEvent.argtypes = [C.c_void_p, C.c_uint, C.c_int, C.c_ulong]
        self.t.XTestFakeMotionEvent.argtypes = [C.c_void_p, C.c_int, C.c_int, C.c_int, C.c_ulong]
        self.t.XTestFakeButtonEvent.argtypes = [C.c_void_p, C.c_uint, C.c_int, C.c_ulong]
        self.d = self.x.XOpenDisplay(display.encode())
        assert self.d, f'Cannot open private display {display}'
        self.root = self.x.XDefaultRootWindow(self.d)

    def atom(self, name):
        return self.x.XInternAtom(self.d, name.encode(), 0)

    def prop(self, window, name):
        actual, fmt, count, left = C.c_ulong(), C.c_int(), C.c_ulong(), C.c_ulong()
        data = C.POINTER(C.c_ubyte)()
        self.x.XGetWindowProperty(self.d, window, self.atom(name), 0, 4096, 0, 0,
            C.byref(actual), C.byref(fmt), C.byref(count), C.byref(left), C.byref(data))
        try:
            if not data:
                return []
            if fmt.value == 32:
                values = C.cast(data, C.POINTER(C.c_ulong))
                return [values[i] for i in range(count.value)]
            return C.string_at(data, count.value)
        finally:
            if data:
                self.x.XFree(data)

    def find(self, pid):
        return next((w for w in self.prop(self.root, '_NET_CLIENT_LIST')
                     if self.prop(w, '_NET_WM_PID') == [pid]), None)

    def focus(self, window):
        self.x.XRaiseWindow(self.d, window)
        self.x.XSetInputFocus(self.d, window, 2, 0)
        self.x.XFlush(self.d)

    def key(self, key, *modifiers):
        codes = [self.x.XKeysymToKeycode(self.d, self.x.XStringToKeysym(n.encode()))
                 for n in (*modifiers, key)]
        assert all(codes), (key, modifiers)
        for code in codes:
            self.t.XTestFakeKeyEvent(self.d, code, 1, 0)
        for code in reversed(codes):
            self.t.XTestFakeKeyEvent(self.d, code, 0, 0)
        self.x.XFlush(self.d)

    def type_ascii(self, text):
        special = {'/': ('slash', ()), '.': ('period', ()), '-': ('minus', ()),
                   '_': ('minus', ('Shift_L',)), ':': ('semicolon', ('Shift_L',)),
                   '@': ('2', ('Shift_L',)), '|': ('backslash', ('Shift_L',)),
                   ',': ('comma', ()), '+': ('equal', ('Shift_L',)),
                   ' ': ('space', ())}
        for char in text:
            key, mods = special.get(char, (char.lower(), ('Shift_L',) if char.isupper() else ()))
            self.key(key, *mods)

    def type_text(self, text):
        """Use GTK's real Unicode keyboard input (Ctrl+Shift+U, hex, Space).

        Changing XKB mappings between individual characters races GTK's map
        cache; the native input method is deterministic and preserves XKB.
        """
        for char in text:
            if ord(char) < 128:
                self.type_ascii(char)
            else:
                self.key('u', 'Control_L', 'Shift_L')
                time.sleep(.025)
                self.type_ascii(format(ord(char), 'x'))
                # Space commits the Unicode composition without invoking an
                # explicit Enter-to-choose controller in a search popup.
                self.key('space')
            time.sleep(.025)

    def pointer(self, x, y, button=1):
        self.t.XTestFakeMotionEvent(self.d, -1, x, y, 0)
        self.t.XTestFakeButtonEvent(self.d, button, 1, 0)
        self.t.XTestFakeButtonEvent(self.d, button, 0, 0)
        self.x.XFlush(self.d)

    def origin(self, window):
        x, y, child = C.c_int(), C.c_int(), C.c_ulong()
        self.x.XTranslateCoordinates(self.d, window, self.root, 0, 0,
                                     C.byref(x), C.byref(y), C.byref(child))
        return x.value, y.value

    def resize(self, window, width, height):
        event = XEvent()
        event.client.type = 33
        event.client.display = self.d
        event.client.window = window
        event.client.message_type = self.atom('_NET_MOVERESIZE_WINDOW')
        event.client.format = 32
        event.client.data[:] = [(1 << 8) | (1 << 9) | (1 << 10) | (1 << 11) | (1 << 12),
                                 20, 20, width, height]
        self.x.XSendEvent(self.d, self.root, 0, (1 << 20) | (1 << 19), C.byref(event))
        self.x.XFlush(self.d)

    def geometry(self, window):
        root, x, y = C.c_ulong(), C.c_int(), C.c_int()
        width, height, border, depth = (C.c_uint() for _ in range(4))
        self.x.XGetGeometry(self.d, window, C.byref(root), C.byref(x), C.byref(y),
            C.byref(width), C.byref(height), C.byref(border), C.byref(depth))
        return [width.value, height.value]
