use super::*;
use aevio_persistence::database::entities::staffing::{
    assignments, availability, contracts, freelancer_skills, freelancers, pipeline, requests,
    skills,
};
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, Schema};
use serde_json::json;

fn database() -> DatabaseConnection {
    let db = Database::connect("sqlite::memory:").expect("in-memory database");
    let schema = Schema::new(DbBackend::Sqlite);
    for statement in [
        schema.create_table_from_entity(freelancers::Entity),
        schema.create_table_from_entity(skills::Entity),
        schema.create_table_from_entity(freelancer_skills::Entity),
        schema.create_table_from_entity(availability::Entity),
        schema.create_table_from_entity(requests::Entity),
        schema.create_table_from_entity(assignments::Entity),
        schema.create_table_from_entity(contracts::Entity),
        schema.create_table_from_entity(pipeline::Entity),
    ] {
        db.execute(&statement).expect("staffing schema");
    }
    db
}

#[test]
fn matching_scores_skills_and_available_capacity() {
    let conn = database();
    let freelancer = save_freelancer(
        &conn,
        FreelancerInput {
            id: Some("freelancer-1".into()),
            name: "Ada Example".into(),
            email: None,
            status: "active".into(),
            location: None,
            timezone: "Europe/Berlin".into(),
            seniority: Some("senior".into()),
            bio: None,
        },
    )
    .expect("freelancer");
    save_json_record(&conn, "skill", &json!({"name":"Rust"})).expect("skill");
    save_json_record(&conn, "skill", &json!({"name":"SQLite"})).expect("skill");
    save_json_record(
        &conn,
        "freelancerSkills",
        &json!({"freelancerId": freelancer.id, "skills": ["Rust", "SQLite"]}),
    )
    .expect("freelancer skills");
    save_json_record(
        &conn,
        "availability",
        &json!({
            "freelancerId": "freelancer-1",
            "startDate": "2026-10-21",
            "endDate": "2026-10-31",
            "availableMinutes": 2400,
            "status": "available"
        }),
    )
    .expect("availability");
    save_json_record(
        &conn,
        "request",
        &json!({
            "id": "request-1",
            "role": "Rust engineer",
            "skills": ["Rust", "SQLite"],
            "startDate": "2026-10-21",
            "endDate": "2026-10-31",
            "requestedMinutes": 1200,
            "status": "open"
        }),
    )
    .expect("request");

    let matches = match_request(&conn, "request-1").expect("matches");
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].freelancer_id, "freelancer-1");
    assert_eq!(matches[0].score, 1.0);
    assert_eq!(matches[0].available_minutes, 2400);
}
