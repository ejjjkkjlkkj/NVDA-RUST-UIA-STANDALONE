#!/usr/bin/env python3
import gi

gi.require_version("Gtk", "3.0")
from gi.repository import GLib, Gtk

window = Gtk.Window(title="Rust Screen Reader AT-SPI Fixture")
window.set_default_size(480, 240)
window.connect("destroy", Gtk.main_quit)

box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
box.set_border_width(24)
window.add(box)

heading = Gtk.Label(label="Accessibility integration fixture")
heading.set_accessible_role if False else None
box.pack_start(heading, False, False, 0)

entry = Gtk.Entry()
entry.set_placeholder_text("Editable text fixture")
entry.set_text("initial text")
box.pack_start(entry, False, False, 0)

button = Gtk.Button(label="Start")
box.pack_start(button, False, False, 0)

status = Gtk.Label(label="waiting")
box.pack_start(status, False, False, 0)


def mutate_once():
    entry.grab_focus()
    entry.set_text("changed accessible text")
    entry.set_position(len(entry.get_text()))
    status.set_text("text changed")
    return False


def mutate_twice():
    button.set_label("Ready")
    button.grab_focus()
    status.set_text("focus changed")
    return False


GLib.timeout_add(1200, mutate_once)
GLib.timeout_add(2400, mutate_twice)
GLib.timeout_add(4500, Gtk.main_quit)

window.show_all()
Gtk.main()
print("GTK_ACCESSIBILITY_FIXTURE = PASS", flush=True)
