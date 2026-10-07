use super::host_of;

#[test]
fn the_paired_computer_named_like_the_server_is_its_host() {
    assert_eq!(host_of(&["soucouyant", "kireserver"], Some("kireserver")), Some(1));
    assert_eq!(host_of(&["soucouyant", " KireServer "], Some("kireserver")), Some(1));
    assert_eq!(host_of(&["soucouyant"], Some("kireserver")), None);
    assert_eq!(host_of(&["kireserver"], None), None);
    assert_eq!(host_of(&["kireserver"], Some("")), None);
}
