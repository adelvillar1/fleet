#!/usr/bin/env python3
"""KI-588 acceptance cell (iii): the update path.

Bump the build hash in place (/bump?to=h2), reload, and measure: the new
worker claims, the app is alive, the previous bot cache is gone, and the
console's cache is untouched. That is what a release looks like to an
installed app — the page asks for new ?v= URLs and the worker script's
bytes change (VERSION is baked into it).

    python3 scripts/bot-pwa-probe/serve.py --port 3196 --hash h1 &
    python3 scripts/bot-pwa-probe/update.py
"""
import argparse
import json
import urllib.request
from playwright.sync_api import sync_playwright


def state(page):
    return page.evaluate("""async () => {
      const regs = await navigator.serviceWorker.getRegistrations();
      const keys = await caches.keys();
      const sizes = {};
      for (const key of keys) { sizes[key] = (await (await caches.open(key)).keys()).length; }
      return {
        controller: navigator.serviceWorker.controller
          ? navigator.serviceWorker.controller.scriptURL : null,
        regs: regs.map((r) => ({ scope: r.scope, script: r.active ? r.active.scriptURL : null })),
        caches: keys, sizes, ready: document.readyState,
        rows: document.querySelectorAll('.row[data-session]').length,
      };
    }""")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--base", default="http://127.0.0.1:3196")
    parser.add_argument("--channel", default="chrome")
    args = parser.parse_args()
    out = {}
    urllib.request.urlopen(args.base + "/bump?to=h1").read()
    with sync_playwright() as p:
        browser = p.chromium.launch(channel=args.channel)
        context = browser.new_context(viewport={"width": 390, "height": 844},
                                      device_scale_factor=2, is_mobile=True, has_touch=True)
        page = context.new_page()
        # The console installs first on this shared origin, so the bump also
        # proves the bot's activate deletes only its own caches.
        page.goto(args.base + "/console", wait_until="domcontentloaded")
        page.evaluate("() => navigator.serviceWorker.ready.then(() => true)")
        page.wait_for_timeout(600)
        page.goto(args.base + "/bot/", wait_until="domcontentloaded")
        page.evaluate("() => navigator.serviceWorker.ready.then(() => true)")
        page.wait_for_timeout(800)
        out["before"] = state(page)

        urllib.request.urlopen(args.base + "/bump?to=h2").read()
        page.reload(wait_until="domcontentloaded")
        page.wait_for_selector(".row[data-session]", timeout=8000)
        page.wait_for_timeout(1500)  # install + activate + claim
        out["after"] = state(page)

        # the app is alive, not frozen: the conversation opens and the
        # composer answers a tap
        page.click(".row[data-session]")
        page.wait_for_selector(".conv .composer", timeout=8000)
        page.wait_for_timeout(300)
        page.click(".composer textarea")
        out["interactive"] = page.evaluate("""() => {
          const cm = document.querySelector('.composer').getBoundingClientRect();
          const hit = document.elementFromPoint(cm.left + cm.width / 2, cm.top + cm.height / 2);
          return { focus: document.activeElement.id || document.activeElement.tagName,
                   hit: hit ? (hit.id || hit.className) : null,
                   dock: Math.round(document.querySelector('.dock').getBoundingClientRect().bottom),
                   h: innerHeight };
        }""")
        browser.close()
    print(json.dumps(out, indent=2))


if __name__ == "__main__":
    main()
