#!/usr/bin/env python3
"""Exercise a real isolated daemon and browser. Requires Python Playwright + Chromium."""
import argparse
import json
import os
import signal
import subprocess
import tempfile
import time
import urllib.error
import urllib.request
from pathlib import Path

from playwright.sync_api import expect, sync_playwright


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--port", type=int, default=41873)
    parser.add_argument("--screenshot", type=Path, default=Path("/tmp/neunode-beta-dashboard.png"))
    args = parser.parse_args()
    binary = str(args.binary.resolve())
    token = "browser-fixture-authority-token-32-characters"
    url = f"http://127.0.0.1:{args.port}"
    with tempfile.TemporaryDirectory(prefix="neunode-dashboard-") as directory:
        env = dict(os.environ, HOME=directory, NEUNODE_API_KEY=token)
        env.pop("NEUNODE_KEYSTORE_KEY", None)
        for command in [["config", "set", "network.listen_addr", "/ip4/127.0.0.1/tcp/0"],
                        ["identity", "create", "--name", "browser-fixture"]]:
            subprocess.run([binary, *command], env=env, check=True, capture_output=True)
        with open(Path(directory) / "daemon.log", "w+") as log:
            daemon = subprocess.Popen([binary, "serve", "--port", str(args.port)], env=env, stdout=log, stderr=log)
            try:
                deadline = time.monotonic() + 15
                while True:
                    if daemon.poll() is not None:
                        log.seek(0)
                        raise RuntimeError(f"Daemon exited: {log.read()}")
                    try:
                        urllib.request.urlopen(f"{url}/api/v1/health", timeout=1).close()
                        break
                    except (urllib.error.URLError, TimeoutError):
                        if time.monotonic() >= deadline:
                            raise RuntimeError("Daemon startup timed out")
                        time.sleep(0.05)
                with sync_playwright() as playwright:
                    browser = playwright.chromium.launch(headless=True)
                    try:
                        page = browser.new_page(viewport={"width": 1400, "height": 1000})
                        # Feed SSE remains open; its HTTP connection is irrelevant to this form test.
                        page.route("**/events/stream", lambda route: route.abort())
                        page.goto(f"{url}/feed")
                        page.wait_for_load_state("networkidle")
                        expect(page.locator("#daemon-access-status")).to_have_text("Read only")
                        page.screenshot(path=str(args.screenshot), full_page=True)
                        page.locator("textarea[name=content]").fill("must not write while locked")
                        with page.expect_response(lambda response: response.url.endswith("/api/feed/post")) as locked:
                            page.get_by_role("button", name="Post", exact=True).click()
                        assert locked.value.status == 401
                        page.get_by_label("Daemon access token").fill(token)
                        page.get_by_role("button", name="Unlock actions for this tab").click()
                        expect(page.locator("#daemon-access-status")).to_have_text("Actions unlocked")
                        expect(page.get_by_label("Daemon access token")).to_have_value("")
                        page.locator("textarea[name=content]").fill("browser signed evidence")
                        page.locator("input[name=tags]").fill("evidence=browser")
                        with page.expect_response(lambda response: response.url.endswith("/api/feed/post")) as posted:
                            page.get_by_role("button", name="Post", exact=True).click()
                        assert posted.value.status == 200
                        response = page.request.get(f"{url}/api/v1/feed")
                        events = response.json()["data"]
                        assert len(events) == 1 and events[0]["content"] == "browser signed evidence"
                        assert events[0]["signature"].startswith("ed25519:")
                        assert {"key": "evidence", "value": "browser"} in events[0]["event"]["tags"]
                        page.reload()
                        page.wait_for_load_state("networkidle")
                        expect(page.locator("#feed-list")).to_contain_text("browser signed evidence")
                        expect(page.locator("#daemon-access-status")).to_have_text("Actions unlocked")
                        page.get_by_role("button", name="Lock", exact=True).click()
                        expect(page.locator("#daemon-access-status")).to_have_text("Read only")
                        page.screenshot(path=str(args.screenshot), full_page=True)
                        print(json.dumps({"passed": ["locked mutations rejected", "unlock authorizes HTMX", "signed post and tags persisted", "reload renders persisted history", "reload preserves tab access", "lock clears access"], "screenshot": str(args.screenshot)}))
                    finally:
                        browser.close()
            finally:
                daemon.send_signal(signal.SIGINT)
                try:
                    daemon.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    daemon.kill()
                    daemon.wait()


if __name__ == "__main__":
    main()
