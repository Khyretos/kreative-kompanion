"""Keycloak admin helpers for test user provisioning with TOTP."""

import base64
import hashlib
import hmac
import json
import os
import secrets
import struct
import time
from urllib.error import HTTPError
from urllib.parse import quote, urlencode
from urllib.request import Request, urlopen

ADMIN_DIR = os.path.expanduser("~/.config/kk-keycloak-admin")


def totp(secret_b32: str, at: float | None = None) -> str:
    """RFC 6238 (HMAC-SHA1, 6 digits, 30 s period). Decode base32 with base64.b32decode (pad with '=' to a multiple of 8, uppercase)."""
    if at is None:
        at = time.time()
    ts = int(at / 30)
    counter = struct.pack(">Q", ts)
    # Pad with '=' to a multiple of 8 bytes, then decode.
    padded = secret_b32.upper() + "=" * (-len(secret_b32.upper()) % 8)
    secret_bytes = base64.b32decode(padded)
    h = hmac.new(secret_bytes, counter, hashlib.sha1).digest()
    o = h[-1] & 0x0F
    code = struct.unpack(">I", h[o : o + 4])[0] & 0x7FFFFFFF
    return f"{code % 10**6:06d}"


def admin_token(base: str) -> str:
    """POST {base}/realms/master/protocol/openid-connect/token with form grant_type=client_credentials, client_id, client_secret from ADMIN_DIR; return access_token."""
    client_id_path = os.path.join(ADMIN_DIR, "client-id")
    client_secret_path = os.path.join(ADMIN_DIR, "client-secret")
    try:
        with open(client_id_path, "r") as f:
            client_id = f.read().strip()
        with open(client_secret_path, "r") as f:
            client_secret = f.read().strip()
    except FileNotFoundError as e:
        raise SystemExit(f"Missing admin credentials in {ADMIN_DIR}: {e}")
    url = f"{base}/realms/master/protocol/openid-connect/token"
    data = {
        "grant_type": "client_credentials",
        "client_id": client_id,
        "client_secret": client_secret,
    }
    req = Request(
        url,
        method="POST",
        data=urlencode(data).encode("utf-8"),
        headers={"Content-Type": "application/x-www-form-urlencoded"},
    )
    try:
        resp = urlopen(req, timeout=10)
        body = json.loads(resp.read().decode("utf-8"))
        return body["access_token"]
    except HTTPError as e:
        raise SystemExit(f"Failed to get admin token: {e.code} {e.reason}")


def kc(
    base: str, realm: str, token: str, method: str, path: str, body=None
) -> tuple[int, dict | None]:
    """urllib request to {base}/admin/realms/{realm}{path} with Bearer token, JSON body; return HTTPError status instead of raising."""
    url = f"{base}/admin/realms/{realm}{path}"
    headers = {"Authorization": f"Bearer {token}", "Content-Type": "application/json"}
    req = Request(url, method=method, headers=headers)
    if body is not None:
        req.data = json.dumps(body).encode("utf-8")
    try:
        resp = urlopen(req, timeout=10)
        status = resp.getcode()
        body_text = resp.read().decode("utf-8")
        parsed = json.loads(body_text) if body_text else None
        return status, parsed
    except HTTPError as e:
        return e.code, None


def make_password() -> str:
    """secrets.token_urlsafe(18) (24 chars)."""
    return secrets.token_urlsafe(18)


def user_id(base: str, realm: str, token: str, name: str) -> str | None:
    """GET /users?username=<name>&exact=true."""
    status, data = kc(
        base, realm, token, "GET", f"/users?username={quote(name)}&exact=true"
    )
    if status == 200 and isinstance(data, list):
        for u in data:
            if u.get("username") == name:
                return u["id"]
    return None


def create_user(base: str, realm: str, token: str, name: str, password: str) -> str:
    """Create an enabled user with a permanent password and the CONFIGURE_TOTP required action.

    Importing an otp credential did not validate in Keycloak 26.7, so the first login sets up the
    one-time code itself (provision_test_user.py reads the secret off that page)."""
    payload = {
        "username": name,
        "enabled": True,
        "emailVerified": True,
        "email": f"{name}@test.kompanion.invalid",
        "firstName": "Test",
        "lastName": "Account",
        "requiredActions": ["CONFIGURE_TOTP"],
        "credentials": [{"type": "password", "value": password, "temporary": False}],
    }
    status, _ = kc(base, realm, token, "POST", "/users", body=payload)
    if status != 201:
        raise SystemExit(f"Failed to create user: {status}")
    uid = user_id(base, realm, token, name)
    if uid is None:
        raise SystemExit("User creation reported success but user not found")
    return uid


def delete_user(base: str, realm: str, token: str, name: str):
    """find the id as above, DELETE /users/<id> (no error when absent)."""
    uid = user_id(base, realm, token, name)
    if uid is None:
        return
    kc(base, realm, token, "DELETE", f"/users/{uid}")


def write_env(path: str, values: dict):
    """replace `KEY=...` lines already in the file, append missing keys, keep every other line, keep the file mode (chmod 600 when creating)."""
    existing_lines = []
    new_lines = []
    mode = None
    try:
        with open(path, "r") as f:
            content = f.read()
            mode = os.stat(path).st_mode
    except FileNotFoundError:
        content = ""
        mode = None

    for line in content.splitlines():
        stripped = line.strip()
        if stripped.startswith("#") or not stripped:
            existing_lines.append(line)
            new_lines.append(line)
            continue
        if "=" in stripped:
            key, _, val = stripped.partition("=")
            key = key.strip()
            val = val.strip()
            if key in values:
                existing_lines.append(f"{key}={values[key]}")
                continue
        existing_lines.append(line)

    for key, val in values.items():
        if key not in [l.split("=")[0].strip() for l in existing_lines if "=" in l]:
            existing_lines.append(f"{key}={val}")

    with open(path, "w") as f:
        f.write("\n".join(existing_lines))
        if existing_lines:
            f.write("\n")
    if mode is not None:
        os.chmod(path, mode)
