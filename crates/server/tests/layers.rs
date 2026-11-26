use serde_json::Value;
use std::process::Command;

#[test]
fn workspace_dependencies_point_inward() {
    let output = Command::new(env!("CARGO"))
        .args(["metadata", "--locked", "--no-deps", "--format-version", "1"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let metadata: Value = serde_json::from_slice(&output.stdout).unwrap();
    for package in metadata["packages"].as_array().unwrap() {
        let name = package["name"].as_str().unwrap();
        let allowed: &[&str] = match name {
            "domain" => &[],
            "application" => &["domain"],
            "infrastructure" | "presentation" => &["application", "domain"],
            "server" => &["application", "presentation", "infrastructure", "domain"],
            _ => panic!("Define the dependency boundary for {name}"),
        };
        for dep in package["dependencies"].as_array().unwrap() {
            let target = dep["name"].as_str().unwrap();
            let is_normal = dep["kind"].is_null();
            match target {
                "axum" => assert!(matches!(name, "presentation" | "server")),
                "sqlx" => assert_eq!(name, "infrastructure"),
                _ => {}
            }
            if is_normal && dep["path"].is_string() {
                assert!(
                    allowed.contains(&target),
                    "{name} must not depend on {target}"
                );
            }
        }
    }
}
