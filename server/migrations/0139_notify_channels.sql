-- NOTIFY: per channel and per event, quiet hours, and Web Push to the installed web app.
ALTER TABLE notification_prefs ADD COLUMN on_approval INTEGER NOT NULL DEFAULT 1;
ALTER TABLE notification_prefs ADD COLUMN mail_on INTEGER NOT NULL DEFAULT 1;
ALTER TABLE notification_prefs ADD COLUMN push_on INTEGER NOT NULL DEFAULT 1;
-- "HH:MM" local time; both empty = no quiet hours. tz_offset: the user's minutes east of UTC.
ALTER TABLE notification_prefs ADD COLUMN quiet_from TEXT NOT NULL DEFAULT '';
ALTER TABLE notification_prefs ADD COLUMN quiet_to TEXT NOT NULL DEFAULT '';
ALTER TABLE notification_prefs ADD COLUMN tz_offset INTEGER NOT NULL DEFAULT 0;
CREATE TABLE web_push_subs (
    id         TEXT PRIMARY KEY,
    user_id    TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    endpoint   TEXT NOT NULL,
    p256dh     TEXT NOT NULL,
    auth       TEXT NOT NULL,
    device     TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL,
    last_ok_at TEXT,
    UNIQUE (user_id, endpoint)
);
