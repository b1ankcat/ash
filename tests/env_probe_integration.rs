use ash::env_probe;

#[test]
fn both_false_gives_empty() {
    let tools: Vec<String> = vec!["git".into()];
    let s = env_probe::collect(false, false, &tools).unwrap();
    assert!(s.text.is_empty());
}

#[test]
fn sys_info_includes_os_arch() {
    let tools: Vec<String> = vec![];
    let s = env_probe::collect(true, false, &tools).unwrap();
    assert!(s.text.contains("os="));
    assert!(s.text.contains("arch="));
    assert!(s.text.contains("cwd="));
}

#[test]
fn env_info_does_not_include_path() {
    let tools: Vec<String> = vec!["git".into()];
    let s = env_probe::collect(false, true, &tools).unwrap();
    // PATH must never be forwarded to the LLM
    assert!(!s.text.contains("PATH="));
}

#[test]
fn custom_tools_to_probe() {
    let tools: Vec<String> = vec!["nonexistent_tool_xyz".into()];
    let s = env_probe::collect(false, true, &tools).unwrap();
    // Nonexistent tool should not appear in output
    assert!(!s.text.contains("nonexistent_tool_xyz"));
}
