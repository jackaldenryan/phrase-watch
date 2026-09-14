from __future__ import annotations

import threading

import AppKit
import Foundation

from phrasewatch.config import load_config
from phrasewatch.listen import listen_microphone
from phrasewatch.notifier import notify
from phrasewatch.paths import CONFIG_PATH, models_ready
from phrasewatch.pipeline import Hit


class AppDelegate(Foundation.NSObject):
    status_item = None
    worker = None
    stop = False
    armed = True
    hit_count = 0

    def applicationDidFinishLaunching_(self, notification) -> None:
        ready, missing = models_ready()
        if not ready:
            _alert("PhraseWatch", "Models missing. Run: phrasewatch download-models")
            AppKit.NSApp.terminate_(None)
            return

        bar = AppKit.NSStatusBar.systemStatusBar()
        self.status_item = bar.statusItemWithLength_(AppKit.NSVariableStatusItemLength)
        self.status_item.button().setTitle_("PW")
        self._rebuild_menu()

        self.stop = False
        self.armed = True
        self.worker = threading.Thread(target=self._run_listen, daemon=True)
        self.worker.start()

    def _rebuild_menu(self) -> None:
        menu = AppKit.NSMenu.alloc().init()
        state = "Listening" if self.armed else "Paused"
        menu.addItemWithTitle_action_keyEquivalent_(
            f"{state}  ·  {self.hit_count} hits", None, ""
        )
        menu.addItem_(AppKit.NSMenuItem.separatorItem())
        toggle = "Pause" if self.armed else "Resume"
        menu.addItemWithTitle_action_keyEquivalent_(toggle, "toggleArmed:", "")
        menu.addItemWithTitle_action_keyEquivalent_("Edit phrases…", "editPhrases:", "")
        menu.addItemWithTitle_action_keyEquivalent_("Open config", "openConfig:", "")
        menu.addItem_(AppKit.NSMenuItem.separatorItem())
        menu.addItemWithTitle_action_keyEquivalent_("Quit PhraseWatch", "quit:", "q")
        self.status_item.setMenu_(menu)

    def _run_listen(self) -> None:
        cfg = load_config()

        def on_hit(hit: Hit) -> None:
            self.hit_count += 1
            AppKit.NSApp.performSelectorOnMainThread_withObject_waitUntilDone_(
                "refreshMenu:", None, False
            )
            if cfg.notify:
                notify(hit)

        def stop_flag() -> bool:
            return self.stop or not self.armed

        while not self.stop:
            if not self.armed:
                AppKit.NSThread.sleepForTimeInterval_(0.3)
                continue
            try:
                listen_microphone(cfg, on_hit, stop_flag=stop_flag)
            except Exception:
                AppKit.NSThread.sleepForTimeInterval_(1.0)

    def refreshMenu_(self, _sender) -> None:
        self._rebuild_menu()

    def toggleArmed_(self, _sender) -> None:
        self.armed = not self.armed
        self._rebuild_menu()

    def editPhrases_(self, _sender) -> None:
        cfg = load_config()
        text = ", ".join(cfg.normalized_phrases())
        AppKit.NSWorkspace.sharedWorkspace().openFile_(str(CONFIG_PATH))
        _alert("Phrases", f"Current phrases:\n{text}\n\nEdit config.json, then Resume.")

    def openConfig_(self, _sender) -> None:
        AppKit.NSWorkspace.sharedWorkspace().openFile_(str(CONFIG_PATH))

    def quit_(self, _sender) -> None:
        self.stop = True
        AppKit.NSApp.terminate_(None)


def _alert(title: str, message: str) -> None:
    alert = AppKit.NSAlert.alloc().init()
    alert.setMessageText_(title)
    alert.setInformativeText_(message)
    alert.runModal()


def run_app() -> None:
    app = AppKit.NSApplication.sharedApplication()
    app.setActivationPolicy_(AppKit.NSApplicationActivationPolicyAccessory)
    delegate = AppDelegate.alloc().init()
    app.setDelegate_(delegate)
    app.run()
