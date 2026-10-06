//! GPU role policy for the A770.
//!
//! Decides when to switch between "coder" (OVMS Coder loaded) and "artist"
//! (Coder unloaded, one studio app: comfyui, heartmula or moss-sfx).
//! Rules from M6-03: never switch while a job runs on the GPU, at most one
//! switch per 5 minutes, coder is the normal state.

pub const MIN_GAP_SECS: i64 = 300;
/// GPU-02: back to coder after the studio has had no job for this long (sooner when a Coder
/// call is waiting).
pub const IDLE_BACK_SECS: i64 = 300;
pub const STUDIO_APPS: &[&str] = &["comfyui", "heartmula", "moss-sfx"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    Coder,
    Artist,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Coder,
    Studio(String),
}

/// What the scheduler sees for the A770 right now.
pub struct View<'a> {
    pub mode: Mode,
    pub switching: bool,
    pub secs_since_switch: Option<i64>,
    pub queued_code: usize,
    pub queued_asset: &'a [String],
    pub running_jobs: usize,
    pub studio_busy: bool,
    pub coder_busy: bool,
    /// GPU-02: Coder calls waiting for the GPU to come back.
    pub coder_waiting: usize,
    /// GPU-02: seconds since the last studio job on this GPU ended (None: none yet).
    pub secs_idle: Option<i64>,
}

/// The studio app an asset job needs: `what` "studio:<app>:..." names it;
/// anything else is comfyui.
pub fn app_for(what: &str) -> String {
    if let Some(app) = what.strip_prefix("studio:").and_then(|rest| rest.split(':').next())
        && STUDIO_APPS.contains(&app)
    {
        return app.to_string();
    }
    "comfyui".to_string()
}

/// Decide the next target for the GPU based on the current view.
pub fn decide(v: &View) -> Option<Target> {
    // Never switch if already switching
    if v.switching {
        return None;
    }

    // Never switch if a job is currently running on the GPU
    if v.running_jobs > 0 {
        return None;
    }

    // Enforce minimum gap between switches
    if v.secs_since_switch.is_some_and(|secs| secs < MIN_GAP_SECS) {
        return None;
    }

    match v.mode {
        Mode::Coder => {
            // Only switch to artist if there are asset jobs waiting, no code jobs, and coder isn't busy
            if !v.queued_asset.is_empty() && v.queued_code == 0 && !v.coder_busy {
                let app = app_for(&v.queued_asset[0]);
                return Some(Target::Studio(app));
            }
            None
        }
        Mode::Artist => {
            // Back to coder when the studio is idle: at once for queued code or a waiting Coder call, else after IDLE_BACK_SECS without a studio job
            if !v.studio_busy && (v.queued_code > 0 || (v.queued_asset.is_empty() && (v.coder_waiting > 0 || v.secs_idle.is_none_or(|s| s >= IDLE_BACK_SECS)))) {
                return Some(Target::Coder);
            }
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a default view with all counts zero and flags false.
    fn view(mode: Mode) -> View<'static> {
        View {
            mode,
            switching: false,
            secs_since_switch: None,
            queued_code: 0,
            queued_asset: &[],
            running_jobs: 0,
            studio_busy: false,
            coder_busy: false,
            coder_waiting: 0,
            secs_idle: None,
        }
    }

    #[test]
    fn test_coder_with_nothing_queued_stays() {
        let v = view(Mode::Coder);
        assert_eq!(decide(&v), None);
    }

    #[test]
    fn test_coder_with_one_asset_job_and_nothing_else() {
        let mut v = view(Mode::Coder);
        let assets = ["studio:heartmula:loop".to_string()];
        v.queued_asset = &assets;
        assert_eq!(decide(&v), Some(Target::Studio("heartmula".to_string())));
    }

    #[test]
    fn test_coder_with_asset_job_and_queued_code_job_stays() {
        let mut v = view(Mode::Coder);
        let assets = ["studio:heartmula:loop".to_string()];
        v.queued_asset = &assets;
        v.queued_code = 1;
        assert_eq!(decide(&v), None);
    }

    #[test]
    fn test_coder_with_asset_job_while_coder_busy_stays() {
        let mut v = view(Mode::Coder);
        let assets = ["studio:heartmula:loop".to_string()];
        v.queued_asset = &assets;
        v.coder_busy = true;
        assert_eq!(decide(&v), None);
    }

    #[test]
    fn test_any_switch_within_299_s_is_refused_at_300_s_allowed() {
        // Artist with nothing queued wants to go back to coder: only after 5 minutes.
        let mut v = view(Mode::Artist);
        v.secs_since_switch = Some(299);
        assert_eq!(decide(&v), None);

        v.secs_since_switch = Some(300);
        assert_eq!(decide(&v), Some(Target::Coder));
    }

    #[test]
    fn test_a_running_job_blocks_every_switch() {
        let mut v = view(Mode::Coder);
        v.running_jobs = 1;
        assert_eq!(decide(&v), None);

        let mut v_artist = view(Mode::Artist);
        v_artist.running_jobs = 1;
        assert_eq!(decide(&v_artist), None);
    }

    #[test]
    fn test_artist_with_empty_queue_and_studio_not_busy_goes_to_coder() {
        let v = view(Mode::Artist);
        assert_eq!(decide(&v), Some(Target::Coder));
    }

    #[test]
    fn test_artist_while_studio_busy_stays() {
        let mut v = view(Mode::Artist);
        v.studio_busy = true;
        assert_eq!(decide(&v), None);
    }

    #[test]
    fn test_artist_with_code_queued_and_studio_idle_goes_to_coder_even_if_assets_queued() {
        let mut v = view(Mode::Artist);
        let assets = ["studio:heartmula:loop".to_string()];
        v.queued_asset = &assets;
        v.queued_code = 1;
        assert_eq!(decide(&v), Some(Target::Coder));
    }

    #[test]
    fn test_app_for_comfyui_default() {
        assert_eq!(app_for("something"), "comfyui");
    }

    #[test]
    fn test_app_for_moss_sfx() {
        assert_eq!(app_for("studio:moss-sfx"), "moss-sfx");
    }

    #[test]
    fn test_app_for_evil_x() {
        assert_eq!(app_for("studio:evil:x"), "comfyui");
    }

    #[test]
    fn test_artist_stays_while_idle_less_than_300_s() {
        let v = View { secs_idle: Some(120), ..view(Mode::Artist) };
        assert_eq!(decide(&v), None);
    }

    #[test]
    fn test_artist_goes_to_coder_after_300_s_idle() {
        let v = View { secs_idle: Some(300), ..view(Mode::Artist) };
        assert_eq!(decide(&v), Some(Target::Coder));
    }

    #[test]
    fn test_artist_goes_to_coder_at_once_when_coder_waits() {
        let v = View { secs_idle: Some(10), coder_waiting: 1, ..view(Mode::Artist) };
        assert_eq!(decide(&v), Some(Target::Coder));
    }

    #[test]
    fn test_artist_keeps_queued_assets_even_when_coder_waits() {
        let assets = vec!["studio:comfyui:z-image-turbo".to_string()];
        let v = View { coder_waiting: 2, queued_asset: &assets, ..view(Mode::Artist) };
        assert_eq!(decide(&v), None);
    }
}
