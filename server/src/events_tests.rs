//! SHARED-LIVE tests: one event to several users, and which events a listener passes on.
use super::*;
use std::collections::HashSet;

fn changed(what: &'static str) -> Event {
    Event::Changed { what, machine_id: None }
}

#[tokio::test]
async fn send_many_reaches_each_user_once() {
    let bus = Bus::new();
    let mut rx = bus.subscribe();
    bus.send_many(&["u1".to_string(), "u2".to_string()], changed("tasks"));
    let (a, _) = rx.recv().await.unwrap();
    let (b, _) = rx.recv().await.unwrap();
    assert_eq!((a.as_str(), b.as_str()), ("u1", "u2"));
    assert!(rx.try_recv().is_err());
}

#[test]
fn a_listener_passes_its_own_events_and_shared_list_changes() {
    let collab: HashSet<String> = ["u1".to_string(), "u2".to_string()].into();
    assert!(passes("u2", &collab, "u2", &changed("settings")));
    assert!(passes("u2", &collab, ALL, &changed("settings")));
    // A collaborator's change to a shared list reloads that list here too.
    for what in ["tasks", "projects", "chats", "project-assets"] {
        assert!(passes("u2", &collab, "u1", &changed(what)), "{what}");
    }
    // Private lists, notifications and chat text stay with their user.
    for what in ["settings", "machines", "access", "lessons", "actions"] {
        assert!(!passes("u2", &collab, "u1", &changed(what)), "{what}");
    }
    let notify = Event::Notify { title: "t".into(), state: "done", url: "/".into() };
    assert!(!passes("u2", &collab, "u1", &notify));
    let delta = Event::MessageDelta { message_id: "m".into(), chat_id: "c".into(), text: "x".into(), done: false };
    assert!(!passes("u2", &collab, "u1", &delta));
    // Someone who shares nothing gets nothing.
    assert!(!passes("u2", &collab, "u9", &changed("tasks")));
}
