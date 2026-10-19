#!/bin/sh
# shellcheck source=/dev/null
. "$(dirname "$0")/common.sh"
F=server/src/sharing.rs
need $F 'pub async fn chat_audience(db: &SqlitePool, chat_id: &str) -> sqlx::Result<Vec<String>>'
need $F 'pub async fn collaborators(db: &SqlitePool, user_id: &str) -> sqlx::Result<Vec<String>>'
need $F 'pub async fn chat_editor('
ctest sharing:: $F
echo "ok: $F"
