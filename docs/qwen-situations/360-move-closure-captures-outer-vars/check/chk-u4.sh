#!/bin/sh
# shellcheck source=/dev/null
. "$(dirname "$0")/common.sh"
F=server/src/api.rs
need $F 'crate::sharing::chat_audience(&s.db, &chat_id)'
# answer() and send() talk to the chat's audience, not one user
body=$(awk '/^async fn answer\(/{p=1} p&&/^}/{print; exit} p' $F)
printf '%s\n' "$body" | grep -nE 'bus\.send\(&(user_id|uid),' | sed "s|^|error: $F: answer() still sends to one user at line |" | head -5
printf '%s\n' "$body" | grep -qE 'bus\.send\(&(user_id|uid),' && exit 1
send=$(awk '/^pub async fn send\(/{p=1} p&&/^}/{print; exit} p' $F)
printf '%s\n' "$send" | grep -nE 'bus\.send\(&u\.id, Event::Message' | sed "s|^|error: $F: send() still sends a message to the writer only: |" | head -5
printf '%s\n' "$send" | grep -qE 'bus\.send\(&u\.id, Event::Message' && exit 1
need $F 'tokio::spawn(answer(s.clone(), owner_id.clone(), chat_id, role, owner_id == u.id));'
ctest api:: $F
echo "ok: $F"
