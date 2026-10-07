use super::*;
use serde_json::json;

#[test]
fn limits_per_kind() {
    assert_eq!(limit("sfx"), Duration::from_secs(300));
    assert_eq!(limit("music"), Duration::from_secs(900));
    assert_eq!(limit("comfyui"), Duration::from_secs(2100));
    assert_eq!(limit("other"), Duration::from_secs(2100));
}

#[test]
fn progress_fails_only_after_the_stall() {
    let t0 = Instant::now();
    let mut p = Progress::new(Duration::from_secs(90), t0);
    assert!(p.tick(false, t0 + Duration::from_secs(60)).is_ok());
    assert!(p.tick(true, t0 + Duration::from_secs(80)).is_ok());
    // alive at 80 s: the clock starts again there
    assert!(p.tick(false, t0 + Duration::from_secs(160)).is_ok());
    let e = p.tick(false, t0 + Duration::from_secs(171)).unwrap_err().to_string();
    assert!(e.contains("90 s"), "{e}");
}

#[test]
fn comfy_queue_lists_the_prompt() {
    let q = json!({"queue_running": [[3, "abc", {}]], "queue_pending": [[4, "def", {}]]});
    assert!(in_queue(&q, "abc"));
    assert!(in_queue(&q, "def"));
    assert!(!in_queue(&q, "xyz"));
    assert!(!in_queue(&json!({}), "abc"));
}

#[test]
fn audio_health_is_alive_only_while_busy() {
    assert!(audio_alive(&json!({"loaded": true, "busy": true})));
    assert!(!audio_alive(&json!({"loaded": true, "busy": false})));
    assert!(!audio_alive(&json!({})));
}

#[tokio::test]
async fn guard_passes_results_through() {
    let w = watch("run-ok", "gpu-a");
    let r = guard(&w, Duration::from_secs(5), async { Ok::<_, anyhow::Error>(7) }).await;
    assert_eq!(r.unwrap(), 7);
}

#[tokio::test]
async fn guard_stops_at_the_limit() {
    let w = watch("run-slow", "gpu-b");
    let r = guard(&w, Duration::from_millis(50), async {
        tokio::time::sleep(Duration::from_secs(5)).await;
        Ok::<_, anyhow::Error>(())
    })
    .await;
    assert!(r.unwrap_err().to_string().contains("time limit"));
}

#[tokio::test]
async fn studio_off_stops_runs_on_its_gpus_only() {
    let a = watch("run-c", "gpu-c");
    let b = watch("run-d", "gpu-d");
    assert_eq!(stop_gpus(&["gpu-c".to_string()]), 1);
    let r = guard(&a, Duration::from_secs(5), async {
        tokio::time::sleep(Duration::from_secs(5)).await;
        Ok::<_, anyhow::Error>(())
    })
    .await;
    assert!(r.unwrap_err().to_string().contains("Studio was turned off"));
    // the other GPU's run keeps going
    let r = guard(&b, Duration::from_millis(100), async { Ok::<_, anyhow::Error>(1) }).await;
    assert_eq!(r.unwrap(), 1);
}

#[test]
fn a_finished_run_is_forgotten() {
    {
        let _w = watch("run-e", "gpu-e");
    }
    assert_eq!(stop_gpus(&["gpu-e".to_string()]), 0);
}
