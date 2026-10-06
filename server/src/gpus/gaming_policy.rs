//! GPU policy logic for Kompanion server.

use serde::Serialize;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum GpuMode {
    Studio,
    Gaming,
    Auto,
}

impl GpuMode {
    pub fn parse(s: &str) -> Option<GpuMode> {
        match s {
            "studio" => Some(GpuMode::Studio),
            "gaming" => Some(GpuMode::Gaming),
            "auto" => Some(GpuMode::Auto),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            GpuMode::Studio => "studio",
            GpuMode::Gaming => "gaming",
            GpuMode::Auto => "auto",
        }
    }
}

pub const IDLE_STOP_MIN: i64 = 15;
pub const GAME_COOLDOWN: Duration = Duration::from_secs(600);

pub fn effective(
    mode: GpuMode,
    gaming: bool,
    since_game_end: Option<Duration>,
) -> GpuMode {
    if mode == GpuMode::Auto {
        if gaming || since_game_end.map_or(false, |d| d < GAME_COOLDOWN) {
            return GpuMode::Gaming;
        }
    }
    mode
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Nothing,
    StopAll { unload: bool },
}

pub fn decide(
    effective: GpuMode,
    apps_up: bool,
    ollama_loaded: bool,
    idle_long: bool,
) -> Action {
    match effective {
        GpuMode::Gaming => {
            if apps_up || ollama_loaded {
                Action::StopAll { unload: true }
            } else {
                Action::Nothing
            }
        }
        GpuMode::Auto => {
            if apps_up && idle_long {
                Action::StopAll { unload: false }
            } else {
                Action::Nothing
            }
        }
        GpuMode::Studio => Action::Nothing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_turns_gaming_while_a_game_runs() {
        assert_eq!(effective(GpuMode::Auto, true, None), GpuMode::Gaming);
        assert_eq!(effective(GpuMode::Auto, false, None), GpuMode::Auto);
    }

    #[test]
    fn auto_stays_gaming_for_the_cooldown() {
        assert_eq!(
            effective(GpuMode::Auto, false, Some(Duration::from_secs(300))),
            GpuMode::Gaming
        );
        assert_eq!(
            effective(GpuMode::Auto, false, Some(Duration::from_secs(601))),
            GpuMode::Auto
        );
    }

    #[test]
    fn fixed_modes_ignore_games() {
        assert_eq!(effective(GpuMode::Studio, true, None), GpuMode::Studio);
        assert_eq!(effective(GpuMode::Gaming, false, None), GpuMode::Gaming);
    }

    #[test]
    fn gaming_stops_and_unloads() {
        assert_eq!(decide(GpuMode::Gaming, true, false, false), Action::StopAll { unload: true });
        assert_eq!(decide(GpuMode::Gaming, false, true, false), Action::StopAll { unload: true });
    }

    #[test]
    fn gaming_with_nothing_up_does_nothing() {
        assert_eq!(decide(GpuMode::Gaming, false, false, true), Action::Nothing);
    }

    #[test]
    fn auto_stops_after_idle() {
        assert_eq!(decide(GpuMode::Auto, true, true, true), Action::StopAll { unload: false });
    }

    #[test]
    fn auto_keeps_apps_while_busy() {
        assert_eq!(decide(GpuMode::Auto, true, false, false), Action::Nothing);
        assert_eq!(decide(GpuMode::Auto, false, true, true), Action::Nothing);
    }

    #[test]
    fn studio_never_stops() {
        assert_eq!(decide(GpuMode::Studio, true, true, true), Action::Nothing);
    }

    #[test]
    fn modes_parse_and_print() {
        for m in [GpuMode::Studio, GpuMode::Gaming, GpuMode::Auto] {
            assert_eq!(GpuMode::parse(m.as_str()), Some(m));
        }
        assert_eq!(GpuMode::parse("off"), None);
    }
}
