//! The Windows logon task that keeps the runner running (RUNNER-WIN): the task definition and the
//! schtasks arguments. Plain text building, so it is tested on Linux; `setup::install_service`
//! runs it on Windows.

#[cfg_attr(not(windows), allow(dead_code))]
pub const TASK_NAME: &str = "KompanionRunner";

/// Escape special XML characters to their entity equivalents.
/// Order matters: escape '&' first, then '<', '>', '"', "'".
#[cfg_attr(not(windows), allow(dead_code))]
pub fn xml_escape(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

/// Build a Windows Task Scheduler 1.2 XML definition for the runner.
/// Uses UTF-16 LE with BOM.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn task_xml(exe: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <Triggers>
    <LogonTrigger>
      <Enabled>true</Enabled>
    </LogonTrigger>
  </Triggers>
  <Principals>
    <Principal id="Author">
      <LogonType>InteractiveToken</LogonType>
      <RunLevel>LeastPrivilege</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <StartWhenAvailable>true</StartWhenAvailable>
    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>
    <RestartOnFailure>
      <Interval>PT1M</Interval>
      <Count>999</Count>
    </RestartOnFailure>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>{}</Command>
    </Exec>
  </Actions>
</Task>"#,
        xml_escape(exe)
    )
}

/// Convert a string to a UTF-16 LE byte vector with BOM.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn utf16le_bom(s: &str) -> Vec<u8> {
    let mut bytes = vec![0xFF, 0xFE];
    for unit in s.encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    bytes
}

/// Build the arguments to create the task via `schtasks`.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn create_args(xml_path: &str) -> Vec<String> {
    ["/Create", "/TN", TASK_NAME, "/XML", xml_path, "/F"].map(String::from).to_vec()
}

/// Build the arguments to run the task via `schtasks`.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn run_args() -> Vec<String> {
    ["/Run", "/TN", TASK_NAME].map(String::from).to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xml_escape_handles_the_five_characters() {
        assert_eq!(xml_escape("a&b<c>d\"e'f"), "a&amp;b&lt;c&gt;d&quot;e&apos;f");
    }

    #[test]
    fn task_xml_starts_at_logon_and_restarts_on_failure() {
        let x = task_xml("C:\\Users\\k & co\\kompanion-runner.exe");
        assert!(x.contains("<LogonTrigger>"));
        assert!(x.contains("<Command>C:\\Users\\k &amp; co\\kompanion-runner.exe</Command>"));
        assert!(x.contains("<RestartOnFailure>") && x.contains("<Interval>PT1M</Interval>") && x.contains("<Count>999</Count>"));
        assert!(x.contains("<ExecutionTimeLimit>PT0S</ExecutionTimeLimit>"));
        assert!(x.contains("<MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>"));
        assert!(x.contains("<DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>"));
        assert!(x.contains("<StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>"));
        assert!(x.contains("<RunLevel>LeastPrivilege</RunLevel>"));
        assert!(x.starts_with("<?xml"));
    }

    #[test]
    fn utf16le_has_a_byte_order_mark() {
        assert_eq!(utf16le_bom("aé"), vec![0xFF, 0xFE, b'a', 0, 0xE9, 0]);
    }

    #[test]
    fn schtasks_arguments() {
        assert_eq!(create_args("C:\\t.xml"), vec!["/Create", "/TN", "KompanionRunner", "/XML", "C:\\t.xml", "/F"]);
        assert_eq!(run_args(), vec!["/Run", "/TN", "KompanionRunner"]);
        assert_eq!(TASK_NAME, "KompanionRunner");
    }
}
