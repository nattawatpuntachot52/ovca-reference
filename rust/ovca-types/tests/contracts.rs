use ovca_types::{
    AuditPacket, Authority, Decision, Evidence, ExecutionBudget, RepairDirective, ReviewPacket,
    RoleInvocation, RoleResult, Validate, VerificationOutcome, WorkflowEvent,
};
use serde::de::DeserializeOwned;

fn assert_contract<T: DeserializeOwned + Validate>(schema: &str, sample: &str) {
    let schema_value: serde_json::Value =
        serde_json::from_str(schema).expect("schema must be valid JSON");
    assert_eq!(
        schema_value["$schema"],
        "https://json-schema.org/draft/2020-12/schema"
    );

    let value: T = serde_json::from_str(sample).expect("sample must match its Rust shape");
    value
        .validate()
        .expect("sample must satisfy Rust semantics");
}

fn assert_missing_field_rejected<T: DeserializeOwned>(sample: &str, field: &str) {
    let mut value: serde_json::Value = serde_json::from_str(sample).unwrap();
    value
        .as_object_mut()
        .expect("sample must be an object")
        .remove(field)
        .expect("field must exist in sample");
    assert!(serde_json::from_value::<T>(value).is_err());
}

#[test]
fn authority_schema_and_sample_are_semantically_valid() {
    assert_contract::<Authority>(
        include_str!("../../../contracts/authority.v1.schema.json"),
        include_str!("../../../contracts/samples/authority.v1.sample.json"),
    );
}

#[test]
fn evidence_schema_and_sample_are_semantically_valid() {
    assert_contract::<Evidence>(
        include_str!("../../../contracts/evidence.v1.schema.json"),
        include_str!("../../../contracts/samples/evidence.v1.sample.json"),
    );
}

#[test]
fn decision_schema_and_sample_are_semantically_valid() {
    assert_contract::<Decision>(
        include_str!("../../../contracts/decision.v1.schema.json"),
        include_str!("../../../contracts/samples/decision.v1.sample.json"),
    );
}

#[test]
fn role_invocation_schema_and_sample_are_semantically_valid() {
    assert_contract::<RoleInvocation>(
        include_str!("../../../contracts/role_invocation.v1.schema.json"),
        include_str!("../../../contracts/samples/role_invocation.v1.sample.json"),
    );
}

#[test]
fn role_result_schema_and_sample_are_semantically_valid() {
    assert_contract::<RoleResult>(
        include_str!("../../../contracts/role_result.v1.schema.json"),
        include_str!("../../../contracts/samples/role_result.v1.sample.json"),
    );
}

#[test]
fn verification_outcome_schema_and_sample_are_semantically_valid() {
    assert_contract::<VerificationOutcome>(
        include_str!("../../../contracts/verification_outcome.v1.schema.json"),
        include_str!("../../../contracts/samples/verification_outcome.v1.sample.json"),
    );
}

#[test]
fn review_packet_schema_and_sample_are_semantically_valid() {
    assert_contract::<ReviewPacket>(
        include_str!("../../../contracts/review_packet.v1.schema.json"),
        include_str!("../../../contracts/samples/review_packet.v1.sample.json"),
    );
}

#[test]
fn audit_packet_schema_and_sample_are_semantically_valid() {
    assert_contract::<AuditPacket>(
        include_str!("../../../contracts/audit_packet.v1.schema.json"),
        include_str!("../../../contracts/samples/audit_packet.v1.sample.json"),
    );
}

#[test]
fn repair_directive_schema_and_sample_are_semantically_valid() {
    assert_contract::<RepairDirective>(
        include_str!("../../../contracts/repair_directive.v1.schema.json"),
        include_str!("../../../contracts/samples/repair_directive.v1.sample.json"),
    );
}

#[test]
fn execution_budget_schema_and_sample_are_semantically_valid() {
    assert_contract::<ExecutionBudget>(
        include_str!("../../../contracts/execution_budget.v1.schema.json"),
        include_str!("../../../contracts/samples/execution_budget.v1.sample.json"),
    );
}

#[test]
fn execution_budget_reserves_the_owner_escalation_path() {
    let too_small = ExecutionBudget {
        contract_version: "1.0".into(),
        max_repair_attempts: 1,
        max_events: 13,
    };
    assert!(too_small.validate().is_err());

    let sufficient = ExecutionBudget {
        max_events: 14,
        ..too_small
    };
    sufficient
        .validate()
        .expect("one repair plus owner escalation requires fourteen events");
}

#[test]
fn workflow_event_schema_and_sample_are_semantically_valid() {
    assert_contract::<WorkflowEvent>(
        include_str!("../../../contracts/workflow_event.v1.schema.json"),
        include_str!("../../../contracts/samples/workflow_event.v1.sample.json"),
    );
}

#[test]
fn schema_required_collection_fields_are_required_on_the_rust_wire() {
    assert_missing_field_rejected::<Authority>(
        include_str!("../../../contracts/samples/authority.v1.sample.json"),
        "constraints",
    );
    assert_missing_field_rejected::<RoleInvocation>(
        include_str!("../../../contracts/samples/role_invocation.v1.sample.json"),
        "input_refs",
    );
    assert_missing_field_rejected::<ReviewPacket>(
        include_str!("../../../contracts/samples/review_packet.v1.sample.json"),
        "findings",
    );
    assert_missing_field_rejected::<AuditPacket>(
        include_str!("../../../contracts/samples/audit_packet.v1.sample.json"),
        "findings",
    );
}
