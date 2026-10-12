//! KG-10 tests: project settings (repo, build, test, run, deploy) shared by every project type.
use super::*;
use crate::scheduler;

#[test]
fn the_two_presets_fill_the_same_fields() {
    let software = Settings::preset("software").expect("software preset");
    let game = Settings::preset("game").expect("game preset");
    assert_eq!(software.preset, "software");
    assert_eq!(game.preset, "game");
    assert_eq!(game.engine, "kk-engine");
    for s in [&software, &game] {
        assert!(s.validate().is_ok());
        assert!(s.command("build").is_some() && s.command("test").is_some(), "build and test are set in {}", s.preset);
    }
    assert!(game.screenshots.len() > 1, "the game preset names a screenshot folder");
    assert!(Settings::preset("chat").is_none());
    assert_eq!(presets().len(), 2);
}

#[test]
fn parse_defaults_everything_it_cannot_read() {
    assert_eq!(Settings::parse(""), Settings::default());
    assert_eq!(Settings::parse("not json"), Settings::default());
    let s = Settings::parse(r#"{"preset":"game","build":"make","extra":1}"#);
    assert_eq!(s.preset, "game");
    assert_eq!(s.build, "make");
    assert_eq!(s.test, "");
}

#[test]
fn a_blank_command_is_not_a_command() {
    let s = Settings { build: "  ".into(), test: " make test ".into(), ..Default::default() };
    assert_eq!(s.command("build"), None);
    assert_eq!(s.command("test"), Some("make test"));
    assert_eq!(s.command("deploy"), None);
    assert_eq!(s.command("nonsense"), None);
}

#[test]
fn validation_refuses_odd_presets_and_multi_line_commands() {
    assert!(Settings { preset: "chat".into(), ..Default::default() }.validate().is_err());
    assert!(Settings { preset: String::new(), ..Default::default() }.validate().is_ok());
    assert!(Settings { build: "make\nrm -rf /".into(), ..Default::default() }.validate().is_err());
    assert!(Settings { test: "a\0b".into(), ..Default::default() }.validate().is_err());
    assert!(Settings { run: "x".repeat(501), ..Default::default() }.validate().is_err());
    assert!(Settings { screenshots: "../out".into(), ..Default::default() }.validate().is_err());
    assert!(Settings { screenshots: "/abs/out".into(), ..Default::default() }.validate().is_err());
    assert!(Settings { screenshots: "shots/latest".into(), ..Default::default() }.validate().is_ok());
}

#[test]
fn the_planner_brief_names_what_is_set_and_nothing_else() {
    assert_eq!(Settings::default().brief(None), "");
    let b = Settings::preset("game").unwrap().brief(Some("/work/fox"));
    assert!(b.contains("/work/fox"));
    assert!(b.contains("kk-engine"));
    assert!(b.contains("Build:") && b.contains("Test:"));
    assert!(!b.contains("Deploy:"), "an empty deploy command is left out");
    let s = Settings { deploy: "sh tools/deploy.sh".into(), ..Default::default() };
    assert!(s.brief(None).contains("Deploy: sh tools/deploy.sh"));
}

#[test]
fn software_and_game_projects_reach_the_scheduler_the_same_way() {
    for name in ["software", "game"] {
        let dev = Settings::preset(name).unwrap();
        let mut cfg = scheduler::Config { enabled: true, ..Default::default() };
        apply_to_scheduler(&mut cfg, &dev, Some("m1"), Some("/work/app"));
        assert!(cfg.enabled, "settings never switch the scheduler on or off");
        assert_eq!(cfg.machines.get("m1").map(String::as_str), Some("/work/app"), "{name}: the repo is the computer and folder");
        assert_eq!(Some(cfg.check.as_str()), dev.command("test"), "{name}: the test command is the check");
    }
}

#[test]
fn the_schedulers_own_values_win_over_the_project_settings() {
    let dev = Settings::preset("software").unwrap();
    let mut cfg = scheduler::Config { enabled: true, check: "make verify".into(), ..Default::default() };
    cfg.machines.insert("m2".into(), "/other".into());
    apply_to_scheduler(&mut cfg, &dev, Some("m1"), Some("/work/app"));
    assert_eq!(cfg.check, "make verify");
    assert_eq!(cfg.machines.len(), 1);
    assert_eq!(cfg.machines.get("m2").map(String::as_str), Some("/other"));
}

#[test]
fn no_repo_and_no_test_command_leave_the_scheduler_config_alone() {
    let mut cfg = scheduler::Config { enabled: true, ..Default::default() };
    apply_to_scheduler(&mut cfg, &Settings::default(), None, None);
    assert!(cfg.machines.is_empty());
    assert_eq!(cfg.check, "");
    apply_to_scheduler(&mut cfg, &Settings::default(), Some("m1"), None);
    assert!(cfg.machines.is_empty(), "a computer without a folder is not a place to run");
}

#[test]
fn a_result_is_stored_per_command_and_kept_short() {
    let mut s = Settings::default();
    s.record("test", true, "2026-10-12T06:00:00Z", &"line\n".repeat(500));
    let r = s.last.get("test").expect("stored");
    assert!(r.ok);
    assert!(r.summary.len() <= 600, "summary is capped");
    s.record("test", false, "2026-10-12T07:00:00Z", "boom");
    assert!(!s.last["test"].ok, "the newest result replaces the old one");
    assert_eq!(s.last.len(), 1);
}
