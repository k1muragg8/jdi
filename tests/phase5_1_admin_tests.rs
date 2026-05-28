use pendulum_kelly_cli::models::WebAdminAuditRecord;
use pendulum_kelly_cli::storage::web_audit_store;
use std::fs;

#[test]
fn test_web_audit_storage() {
    let dir_path = "data/test_tmp_audit";
    let _ = fs::create_dir_all(dir_path);
    let file_path = format!("{}/web_audit.json", dir_path);
    let _ = fs::remove_file(&file_path);

    let record = WebAdminAuditRecord {
        audit_id: "test_id".to_string(),
        timestamp: "2026-05-27 10:00:00".to_string(),
        actor: "local_web".to_string(),
        actor_user_id: Some("user1".to_string()),
        target_user_id: Some("user1".to_string()),
        portfolio_id: Some("p1".to_string()),
        role: Some("owner".to_string()),
        action: "test_action".to_string(),
        target_file: "test.json".to_string(),
        target_id: Some("target1".to_string()),
        old_value_summary: "old".to_string(),
        new_value_summary: "new".to_string(),
        status: "success".to_string(),
        note: None,
    };

    web_audit_store::add_audit_record(&file_path, record.clone()).unwrap();

    let log = web_audit_store::load_web_audit(&file_path).unwrap();
    assert_eq!(log.records.len(), 1);
    assert_eq!(log.records[0].audit_id, "test_id");
    assert_eq!(log.records[0].actor_user_id, Some("user1".to_string()));
}
