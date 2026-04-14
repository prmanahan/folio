mod common;

#[tokio::test]
async fn test_get_profile_returns_public_fields() {
    let server = common::test_app();
    let response = server.get("/api/profile").await;
    response.assert_status_ok();

    let body: serde_json::Value = response.json();
    assert_eq!(body["name"], "Alex Rivera");
    assert_eq!(body["email"], "alex@example.com");
    assert_eq!(body["availability_status"], "open");

    // Pitch split is exposed publicly
    assert!(
        body.get("pitch_short").is_some(),
        "pitch_short must be in public profile"
    );
    assert!(
        body.get("pitch_long").is_some(),
        "pitch_long must be in public profile"
    );
    assert!(
        !body["pitch_short"].as_str().unwrap_or("").is_empty(),
        "seed pitch_short must be non-empty"
    );

    // Old elevator_pitch is gone from the API surface
    assert!(
        body.get("elevator_pitch").is_none(),
        "elevator_pitch must not appear post-migration"
    );

    // Verify AI-tier and Private-tier fields are NOT present
    assert!(body.get("salary_min").is_none()); // Private
    assert!(body.get("salary_max").is_none()); // Private
    assert!(body.get("career_narrative").is_none()); // AI
    assert!(body.get("looking_for").is_none()); // AI
}
