#!/usr/bin/env python3
"""KI-588 acceptance cells (i), (ii), (iv) — measured in a real browser.

    python3 scripts/bot-pwa-probe/serve.py --port 3196 --hash h1 &
    python3 scripts/bot-pwa-probe/probe.py after      # the installed app
    python3 scripts/bot-pwa-probe/probe.py baseline   # the pre-fix page (/bot.html)

`after` opens the app at /bot/; `baseline` opens the pre-fix /bot.html, so
the same script measures the defect and the fix. Every number printed is
read out of the live page.

Cell (i)   controller + registrations + cache contents and sizes
Cell (ii)  offline: this page's retry text and which worker controls it
Cell (iv)  the conversation at 390x844: .conv/.scroll/.dock geometry,
           elementFromPoint on the composer, and a real tap's focus target
           — plus the red-proof: the same page with the .conv rule removed.
"""
import argparse
import json
from playwright.sync_api import sync_playwright


def sw_state(page):
    return page.evaluate("""async () => {
      const regs = await navigator.serviceWorker.getRegistrations();
      const keys = await caches.keys();
      const sizes = {};
      for (const key of keys) { sizes[key] = (await (await caches.open(key)).keys()).length; }
      return {
        controller: navigator.serviceWorker.controller
          ? navigator.serviceWorker.controller.scriptURL : null,
        regs: regs.map((r) => ({ scope: r.scope, script: r.active ? r.active.scriptURL : null })),
        caches: keys, sizes,
      };
    }""")


def layout(page):
    return page.evaluate("""() => {
      const app = document.getElementById('app');
      const conv = document.querySelector('.conv');
      const scroll = document.querySelector('.scroll');
      const dock = document.querySelector('.dock');
      const box = (el) => { if (!el) return null; const r = el.getBoundingClientRect();
        return { top: Math.round(r.top), bottom: Math.round(r.bottom), height: Math.round(r.height) }; };
      const composer = document.querySelector('.composer') || document.querySelector('.dispatch');
      const cb = box(composer);
      const center = composer ? composer.getBoundingClientRect() : null;
      const at = center ? document.elementFromPoint(center.left + center.width / 2,
        Math.min(center.top + center.height / 2, innerHeight - 1)) : null;
      const cs = conv ? getComputedStyle(conv) : null;
      return {
        viewport: { w: innerWidth, h: innerHeight },
        app: { display: app ? getComputedStyle(app).display : null,
               direction: app ? getComputedStyle(app).flexDirection : null, box: box(app) },
        conv: conv ? { display: cs.display, direction: cs.flexDirection, box: box(conv) } : null,
        scroll: scroll ? { box: box(scroll), scrollHeight: scroll.scrollHeight,
                           clientHeight: scroll.clientHeight } : null,
        dock: box(dock), composer: cb,
        hit: at ? (at.id || at.className || at.tagName) : null,
        hitInsideDock: at ? !!(dock && dock.contains(at)) : false,
      };
    }""")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("phase", choices=["baseline", "after"])
    parser.add_argument("--base", default="http://127.0.0.1:3196")
    parser.add_argument("--channel", default="chrome")
    args = parser.parse_args()
    path = "/bot.html" if args.phase == "baseline" else "/bot/"
    out = {}
    with sync_playwright() as p:
        browser = p.chromium.launch(channel=args.channel)
        context = browser.new_context(viewport={"width": 390, "height": 844},
                                      device_scale_factor=2, is_mobile=True, has_touch=True)
        page = context.new_page()

        # ---- the console installs first: its worker at scope / on this origin
        page.goto(args.base + "/console", wait_until="domcontentloaded")
        page.evaluate("() => navigator.serviceWorker.ready.then(() => true)")
        page.wait_for_timeout(600)
        out["console"] = sw_state(page)

        # ---- the bot face, same origin
        page.goto(args.base + path, wait_until="domcontentloaded")
        page.wait_for_selector(".row[data-session]", timeout=8000)
        page.click(".row[data-session]")
        page.wait_for_selector(".conv .composer", timeout=8000)
        page.wait_for_timeout(1200)
        out["bot_sw"] = sw_state(page)
        out["bot_layout"] = layout(page)

        # red-proof: remove the .conv rule and re-measure (the clip returns)
        page.add_style_tag(content=".conv{display:block !important;flex:none !important;min-height:auto !important}")
        page.wait_for_timeout(250)
        out["bot_layout_no_rule"] = layout(page)
        page.reload(wait_until="domcontentloaded")
        page.wait_for_selector(".row[data-session]", timeout=8000)
        page.click(".row[data-session]")
        page.wait_for_selector(".conv .composer", timeout=8000)
        page.wait_for_timeout(300)

        # a tap on the composer answers a node (focus lands in the textarea)
        page.click(".composer textarea")
        out["composer_focus"] = page.evaluate(
            "() => document.activeElement.id || document.activeElement.tagName")

        # ---- offline: the retry page this face owns
        context.set_offline(True)
        page.reload(wait_until="domcontentloaded")
        page.wait_for_timeout(900)
        out["offline_bot"] = page.evaluate("""() => ({
          title: document.title, h1: (document.querySelector('h1') || {}).textContent || null,
          text: (document.body.innerText || '').slice(0, 200),
          controlledBy: navigator.serviceWorker.controller
            ? navigator.serviceWorker.controller.scriptURL : null,
        })""")

        # the console, same offline device: still its own page, never the bot's
        console = context.new_page()
        console.goto(args.base + "/console", wait_until="domcontentloaded")
        console.wait_for_timeout(700)
        out["offline_console"] = console.evaluate("""() => ({
          title: document.title, h1: (document.querySelector('h1') || {}).textContent || null,
          text: (document.body.innerText || '').slice(0, 200),
          controlledBy: navigator.serviceWorker.controller
            ? navigator.serviceWorker.controller.scriptURL : null,
        })""")
        context.set_offline(False)
        browser.close()
    print(json.dumps(out, indent=2))


if __name__ == "__main__":
    main()
