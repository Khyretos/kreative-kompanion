# Test account for automated live checks (AUTH-01)

Kompanion has no signup page and no user screen: accounts are created by the first OIDC sign-in
(Keycloak realm `kreative-kompas`, `allow_new = ["*"]`), and Keycloak makes every user set up a
one-time code (TOTP). A test account therefore has to exist in Keycloak first.

```sh
python3 tools/provision_test_user.py            # create "claude" if missing, then log in
python3 tools/provision_test_user.py --recreate # delete it, new password and TOTP secret
python3 tools/provision_test_user.py --login-only # fresh session from the .env credentials, no admin calls
```

What it does:

1. Gets an admin token with the `claude-admin` client of the master realm (`~/.config/kk-keycloak-admin`, see Services/keycloak/README.md).
2. Creates the user with a random 24-character password and the required action CONFIGURE_TOTP
   (importing an `otp` credential did not validate in Keycloak 26.7).
3. Signs in through the real OIDC flow against the local app (`http://127.0.0.1:8095`): password, sets up the
   code and reads its secret off the page, then replays the callback on the local app, so Kompanion creates the account.
4. Writes `KOMPANION_TEST_USER`, `KOMPANION_TEST_PASS` and `KOMPANION_TEST_TOTP` to `.env` (never printed) and the
   session cookies to `~/.cache/kompanion-test-session.txt` (chmod 600).

Use it: `curl -b ~/.cache/kompanion-test-session.txt -H 'X-Kompanion: 1' http://127.0.0.1:8095/api/...`. For a browser,
run a small local proxy that adds that Cookie header (the cookie is HttpOnly, so the page cannot set it) and open the
proxy's port. Only against the local app: never type these credentials on the public site.

Notes: the app marks cookies `Secure`; the script clears that flag for the plain-http local URL. Keycloak locks a user after
10 failed codes (`DELETE /admin/realms/kreative-kompas/attack-detection/brute-force/users/<id>` clears it). The test user is
a normal user (no `kompanion-admin` role). Delete the Keycloak user and the Kompanion user "claude" to retire it.
