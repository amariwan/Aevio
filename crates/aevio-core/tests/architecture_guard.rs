//! Automated architecture guard for the `aevio-core` domain kernel.
//!
//! Pure domain modules must not reach into persistence, transport or runtime
//! crates. This test fails the build if a known-pure module gains a forbidden
//! import, so dependency drift is caught in CI instead of in review.

use std::fs;
use std::path::PathBuf;

const FORBIDDEN: &[&str] = &["sea_orm", "rusqlite", "reqwest", "tauri", "axum", "tokio::"];

const PURE_MODULES: &[&str] = &[
    "entries/models.rs",
    "entries/builder.rs",
    "settings/breaks.rs",
    "settings/tracking.rs",
    "settings/validation.rs",
    "settings/models.rs",
    "ai/arbiter.rs",
    "ai/calibration.rs",
    "ai/knn.rs",
    "ai/rules.rs",
    "coach/models.rs",
    "coach/policies.rs",
    "coach/assistance.rs",
    "coach/intelligence.rs",
    "coach/interventions.rs",
    "coach/adaptive.rs",
    "coach/dates.rs",
    "coach/focus.rs",
    "coach/legacy.rs",
    "integrations/models.rs",
    "energy/mod.rs",
    "platform/mod.rs",
    "capture/mod.rs",
    "categories/mod.rs",
    "reports/mod.rs",
];

#[test]
fn pure_domain_modules_have_no_forbidden_imports() {
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut violations = Vec::new();
    for rel in PURE_MODULES {
        let path = src.join(rel);
        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => panic!("cannot read {}: {}", path.display(), e),
        };
        for forbidden in FORBIDDEN {
            if content.contains(forbidden) {
                violations.push(format!("{} contains forbidden import '{}'", rel, forbidden));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "Architecture guard failed:\n{}",
        violations.join("\n")
    );
}

#[test]
fn core_does_not_depend_on_services_or_persistence() {
    let cargo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let content = fs::read_to_string(&cargo).expect("read Cargo.toml");
    for forbidden in &[
        "aevio-services",
        "aevio-persistence",
        "aevio-infrastructure",
        "aevio-http",
    ] {
        assert!(
            !content.contains(forbidden),
            "aevio-core must not depend on {}",
            forbidden
        );
    }
}
