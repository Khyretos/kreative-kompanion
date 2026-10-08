"""Provision a test/service user "claude" and log it in to Kompanion (AUTH-01).

Kompanion users exist only through OIDC (Keycloak, which also forces TOTP), so this creates the
Keycloak user with a random password and TOTP secret through the admin API, stores them in .env
(KOMPANION_TEST_USER / _PASS / _TOTP) and signs in through the real OIDC flow.
  python3 tools/provision_test_user.py             # create if missing, then log in
  python3 tools/provision_test_user.py --recreate  # new password and TOTP secret
Admin client secret: ~/.config/kk-keycloak-admin. Then use the cookie jar for live checks:
  curl -b ~/.cache/kompanion-test-session.txt -H 'X-Kompanion: 1' http://127.0.0.1:8095/api/...
"""

import argparse
import base64
import html
import os
import re
import sys
import time
from http.cookiejar import MozillaCookieJar
from urllib.error import HTTPError
from urllib.parse import urlencode, urljoin
from urllib.request import (
    HTTPCookieProcessor,
    HTTPRedirectHandler,
    Request,
    build_opener,
)

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import kc_admin as kc

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


class NoRedirect(HTTPRedirectHandler):
    def redirect_request(self, *a, **k):
        return None


def issuer_parts(toml_path):
    m = re.search(r'^issuer\s*=\s*"([^"]+)"', open(toml_path).read(), re.M)
    if not m or "/realms/" not in m.group(1):
        raise SystemExit(
            "no OIDC issuer like https://host/realms/<realm> in " + toml_path
        )
    base, realm = m.group(1).rsplit("/realms/", 1)
    return base, realm


def read_env(path):
    out = {}
    for line in open(path):
        k, sep, v = line.strip().partition("=")
        if sep and not k.startswith("#"):
            out[k.strip()] = v.strip().strip('"')
    return out


class Browser:
    """One cookie jar, redirects handled by hand so the Kompanion callback can be replayed locally."""

    def __init__(self):
        self.jar = MozillaCookieJar()
        self.opener = build_opener(HTTPCookieProcessor(self.jar), NoRedirect)

    def fetch(self, url, form=None):
        data = urlencode(form).encode() if form is not None else None
        try:
            r = self.opener.open(Request(url, data=data), timeout=20)
        except HTTPError as e:
            r = e
        if url.startswith(
            "http://"
        ):  # the app marks cookies Secure; plain-http localhost is fine for a test login
            for c in self.jar:
                c.secure = False
        return (
            r.status if hasattr(r, "status") else r.code,
            r.headers,
            r.read().decode("utf-8", "replace"),
        )

    def follow(self, url, stop=lambda u: False):
        """GET url and follow redirects until a page, or a Location that stop() accepts (returned)."""
        for _ in range(10):
            if stop(url):
                return url, None
            status, headers, body = self.fetch(url)
            loc = headers.get("Location")
            if status in (301, 302, 303, 307) and loc:
                url = urljoin(url, loc)
                continue
            return url, body
        raise SystemExit("too many redirects")


def form_of(page, field):
    """(action, hidden fields) of the form that has an input called field, or None."""
    for f in re.finditer(r"<form\b([^>]*)>(.*?)</form>", page, re.S | re.I):
        if re.search(r'name=["\']%s["\']' % field, f.group(2)):
            action = re.search(r'action=["\']([^"\']*)["\']', f.group(1))
            hidden = {
                n: html.unescape(v)
                for n, v in re.findall(
                    r'<input[^>]*type=["\']hidden["\'][^>]*name=["\'](\w+)["\'][^>]*value=["\']([^"\']*)',
                    f.group(2),
                )
            }
            return html.unescape(action.group(1)), hidden
    return None


def oidc_login(kompanion, name, password, secret, jar_path, save=lambda secret: None):
    """Signs in through the real OIDC flow and saves the Kompanion cookies; returns the TOTP secret
    (base32), set up on the spot when the user still has the CONFIGURE_TOTP required action.
    """
    app = Browser()
    status, headers, _ = app.fetch(kompanion + "/api/auth/oidc/start")
    if status not in (302, 303) or not headers.get("Location"):
        raise SystemExit(f"login failed at start ({status})")
    sso = Browser()
    url, page = sso.follow(headers["Location"])
    step = form_of(page or "", "username")
    if not step:
        raise SystemExit("login failed at the Keycloak sign-in page")
    callback = lambda u: "/api/auth/oidc/callback?" in u
    status, headers, page = sso.fetch(
        step[0], {**step[1], "username": name, "password": password, "credentialId": ""}
    )
    setup = headers.get("Location", "") if status in (302, 303) else ""
    if "required-action" in setup:
        _, page = sso.follow(setup)
        raw = re.search(r'name="totpSecret"[^>]*value="([^"]+)"', page or "")
        if not raw:
            raise SystemExit("login failed: no TOTP setup form")
        secret = base64.b32encode(raw.group(1).encode()).decode().rstrip("=")
        action = form_of(page, "totp")[0]
        save(secret)  # Keycloak will not show it again
        status, headers, page = sso.fetch(
            action,
            {
                "totp": kc.totp(secret),
                "totpSecret": raw.group(1),
                "userLabel": "kompanion-test",
                "submitAction": "Submit",
            },
        )
    for attempt in range(2):
        if status in (302, 303):
            break
        otp = form_of(page, "otp")
        if not otp:
            raise SystemExit("login failed: Keycloak did not accept the password")
        status, headers, page = sso.fetch(
            otp[0], {**otp[1], "otp": kc.totp(secret), "login": "Sign In"}
        )
        if status not in (302, 303) and attempt == 0:
            time.sleep(31 - time.time() % 30)  # a code is single-use per 30 s window
    if status not in (302, 303):
        raise SystemExit("login failed at the one-time code")
    url, _ = sso.follow(urljoin(step[0], headers["Location"]), callback)
    if not callback(url):
        raise SystemExit(
            "login failed: no callback redirect, Keycloak asked for: "
            + re.sub(r"\s+", " ", re.sub(r"<[^>]+>|<script.*?</script>", " ", _ or ""))[
                :200
            ]
        )
    status, _, _ = app.fetch(kompanion + url[url.index("/api/auth/oidc/callback?") :])
    if status not in (302, 303) or not any(
        c.name.startswith("kompanion") or "session" in c.name for c in app.jar
    ):
        raise SystemExit(f"login failed at the callback ({status})")
    app.jar.filename = jar_path
    os.makedirs(os.path.dirname(jar_path), exist_ok=True)
    app.jar.save(ignore_discard=True, ignore_expires=True)
    os.chmod(jar_path, 0o600)
    return secret


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--name", default="claude")
    ap.add_argument("--env", default=os.path.join(ROOT, ".env"))
    ap.add_argument("--toml", default=os.path.join(ROOT, "kompanion.toml"))
    ap.add_argument("--kompanion", default="http://127.0.0.1:8095")
    ap.add_argument(
        "--jar", default=os.path.expanduser("~/.cache/kompanion-test-session.txt")
    )
    ap.add_argument(
        "--recreate",
        action="store_true",
        help="delete the Keycloak user, make new credentials",
    )
    ap.add_argument(
        "--login-only",
        action="store_true",
        help="use the .env credentials, no admin calls",
    )
    a = ap.parse_args()
    env = read_env(a.env) if os.path.exists(a.env) else {}
    have = (
        all(env.get(k) for k in ("KOMPANION_TEST_PASS", "KOMPANION_TEST_TOTP"))
        and env.get("KOMPANION_TEST_USER") == a.name
    )
    if not a.login_only:
        base, realm = issuer_parts(a.toml)
        token = kc.admin_token(base)
        exists = kc.user_id(base, realm, token, a.name) is not None
        if exists and a.recreate:
            kc.delete_user(base, realm, token, a.name)
            exists = False
        if exists and not have:
            raise SystemExit(
                f"{a.name} exists in Keycloak but {a.env} has no credentials: use --recreate"
            )
        if not exists:
            env = {
                "KOMPANION_TEST_USER": a.name,
                "KOMPANION_TEST_PASS": kc.make_password(),
                "KOMPANION_TEST_TOTP": "",
            }
            kc.create_user(base, realm, token, a.name, env["KOMPANION_TEST_PASS"])
            kc.write_env(a.env, env)
    elif not have:
        raise SystemExit("no KOMPANION_TEST_USER/PASS/TOTP in " + a.env)
    oidc_login(
        a.kompanion,
        a.name,
        env["KOMPANION_TEST_PASS"],
        env.get("KOMPANION_TEST_TOTP", ""),
        a.jar,
        lambda secret: kc.write_env(a.env, {"KOMPANION_TEST_TOTP": secret}),
    )
    print(f"logged in as {a.name}; session cookies in {a.jar}")


if __name__ == "__main__":
    main()
