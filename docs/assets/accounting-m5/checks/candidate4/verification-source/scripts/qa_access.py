#!/usr/bin/env python3
"""Small AT-SPI C adapter for an explicitly owned native QA process.

This avoids a GI Atspi typelib requirement; it never selects another app.
The caller owns the private D-Bus session and synthetic Sarv process.
"""
import ctypes as C
import ctypes.util


class Rect(C.Structure):
    _fields_ = [(name, C.c_int) for name in ('x', 'y', 'width', 'height')]


class Access:
    def __init__(self, pid):
        self.pid = pid
        self.a = C.CDLL(ctypes.util.find_library('atspi'))
        self.g = C.CDLL(ctypes.util.find_library('glib-2.0'))
        self.g.g_free.argtypes = [C.c_void_p]
        self.g.g_main_context_pending.argtypes = [C.c_void_p]
        self.g.g_main_context_pending.restype = C.c_int
        self.g.g_main_context_iteration.argtypes = [C.c_void_p, C.c_int]
        self.g.g_main_context_iteration.restype = C.c_int
        signatures = {
            'atspi_init': (C.c_int, []),
            'atspi_set_timeout': (None, [C.c_int, C.c_int]),
            'atspi_get_desktop': (C.c_void_p, [C.c_int]),
            'atspi_accessible_get_process_id': (C.c_uint, [C.c_void_p, C.c_void_p]),
            'atspi_accessible_get_child_count': (C.c_int, [C.c_void_p, C.c_void_p]),
            'atspi_accessible_get_child_at_index': (C.c_void_p, [C.c_void_p, C.c_int, C.c_void_p]),
            'atspi_accessible_get_parent': (C.c_void_p, [C.c_void_p, C.c_void_p]),
            'atspi_accessible_get_name': (C.c_void_p, [C.c_void_p, C.c_void_p]),
            'atspi_accessible_get_role_name': (C.c_void_p, [C.c_void_p, C.c_void_p]),
            'atspi_accessible_get_state_set': (C.c_void_p, [C.c_void_p]),
            'atspi_accessible_clear_cache': (None, [C.c_void_p]),
            'atspi_state_set_contains': (C.c_int, [C.c_void_p, C.c_int]),
            'atspi_accessible_get_action_iface': (C.c_void_p, [C.c_void_p]),
            'atspi_action_do_action': (C.c_int, [C.c_void_p, C.c_int, C.c_void_p]),
            'atspi_accessible_get_component_iface': (C.c_void_p, [C.c_void_p]),
            'atspi_component_grab_focus': (C.c_int, [C.c_void_p, C.c_void_p]),
            'atspi_component_get_extents': (C.POINTER(Rect), [C.c_void_p, C.c_int, C.c_void_p]),
            'atspi_accessible_get_text_iface': (C.c_void_p, [C.c_void_p]),
            'atspi_text_get_text': (C.c_void_p, [C.c_void_p, C.c_int, C.c_int, C.c_void_p]),
        }
        for name, (result, args) in signatures.items():
            function = getattr(self.a, name)
            function.restype, function.argtypes = result, args
        # 1 means the same process has already initialized this client library.
        if self.a.atspi_init() not in (0, 1):
            raise RuntimeError('AT-SPI initialization failed')
        self.a.atspi_set_timeout(1000, 1500)
        self.desktop = self.a.atspi_get_desktop(0)

    def children(self, node):
        return [self.a.atspi_accessible_get_child_at_index(node, i, None)
                for i in range(max(0, self.a.atspi_accessible_get_child_count(node, None)))]

    def app(self):
        self.pump()
        return next((node for node in self.children(self.desktop)
                     if self.a.atspi_accessible_get_process_id(node, None) == self.pid), None)

    def string(self, function, *args):
        value = function(*args)
        if not value:
            return ''
        result = C.string_at(value).decode('utf-8')
        self.g.g_free(value)
        return result

    def tree(self):
        app = self.app()
        if not app:
            return []
        queue, rows = [app], []
        while queue and len(rows) < 8000:
            node = queue.pop(0)
            if not node:
                continue
            self.a.atspi_accessible_clear_cache(node)
            rows.append({'node': node,
                         'name': self.string(self.a.atspi_accessible_get_name, node, None),
                         'role': self.string(self.a.atspi_accessible_get_role_name, node, None)})
            queue.extend(self.children(node))
        return rows

    def find(self, name, roles=None):
        return next((row for row in self.tree()
                     if row['name'] == name and (not roles or row['role'] in roles)), None)

    def focus(self, row):
        iface = self.a.atspi_accessible_get_component_iface(row['node'])
        return bool(iface and self.a.atspi_component_grab_focus(iface, None))

    def pump(self):
        for _ in range(200):
            if not self.g.g_main_context_pending(None):
                break
            self.g.g_main_context_iteration(None, 0)

    def focused(self, row):
        self.pump()
        node = row['node']
        if not node:
            return False
        # Focus belongs to this control. An ancestor container's focused state
        # does not establish that the requested control itself received focus.
        self.a.atspi_accessible_clear_cache(node)
        states = self.a.atspi_accessible_get_state_set(node)
        return bool(states and self.a.atspi_state_set_contains(states, 12))

    def click(self, row):
        iface = self.a.atspi_accessible_get_action_iface(row['node'])
        return bool(iface and self.a.atspi_action_do_action(iface, 0, None))

    def rect(self, row):
        iface = self.a.atspi_accessible_get_component_iface(row['node'])
        rect = self.a.atspi_component_get_extents(iface, 0, None) if iface else None
        if not rect:
            return None
        result = {name: getattr(rect.contents, name) for name in ('x', 'y', 'width', 'height')}
        self.g.g_free(rect)
        return result

    def text(self, row):
        iface = self.a.atspi_accessible_get_text_iface(row['node'])
        return self.string(self.a.atspi_text_get_text, iface, 0, -1, None) if iface else ''
