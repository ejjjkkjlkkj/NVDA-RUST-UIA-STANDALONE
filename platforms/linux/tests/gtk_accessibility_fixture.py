#!/usr/bin/env python3
import gi

gi.require_version("Atk", "1.0")
gi.require_version("Gtk", "3.0")
from gi.repository import Atk, GLib, Gtk

window = Gtk.Window(title="Rust Screen Reader AT-SPI Fixture")
window.set_default_size(480, 240)
window.connect("destroy", Gtk.main_quit)

box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
box.set_border_width(24)
window.add(box)

heading = Gtk.Label(label="Accessibility integration fixture")
box.pack_start(heading, False, False, 0)

entry = Gtk.Entry()
entry.set_placeholder_text("Editable text fixture")
entry.set_text("initial text")
entry.get_accessible().set_accessible_id("fixture-entry")
entry.get_accessible().set_name("Editable text fixture")
box.pack_start(entry, False, False, 0)

button = Gtk.Button(label="Start")
button.get_accessible().set_accessible_id("fixture-button")
box.pack_start(button, False, False, 0)

status = Gtk.Label(label="waiting")
status.get_accessible().set_accessible_id("fixture-status")
box.pack_start(status, False, False, 0)


def mutate_once():
    entry.grab_focus()
    entry.get_accessible().notify_state_change(Atk.StateType.FOCUSED, True)
    entry.set_text("changed accessible text")
    entry.set_position(len(entry.get_text()))
    status.set_text("text changed")
    return False


def mutate_twice():
    entry.get_accessible().notify_state_change(Atk.StateType.FOCUSED, False)
    button.set_label("Ready")
    button.grab_focus()
    button.get_accessible().notify_state_change(Atk.StateType.FOCUSED, True)
    status.set_text("focus changed")
    return False


GLib.timeout_add(1200, mutate_once)
GLib.timeout_add(2400, mutate_twice)
GLib.timeout_add(4500, Gtk.main_quit)

window.show_all()
Gtk.main()
print("GTK_ACCESSIBILITY_FIXTURE = PASS", flush=True)
