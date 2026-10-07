use ovca_types::{
    Decision, DecisionDisposition, FinalResponse, Finding, FindingSeverity, ReviewDecision,
    RoleResultStatus, VerificationStatus, WorkflowEvent, WorkflowEventKind, WorkflowOutcome,
};
use ovca_workflow::{recover_public, WorkflowError, WorkflowPhase, WorkflowReducer};

fn fixture(source: &str) -> Vec<WorkflowEvent> {
    serde_json::from_str(source).expect("fixture must deserialize")
}

#[test]
fn pass_fixture_reaches_a_public_pass_response() {
    let events = fixture(include_str!("../../../examples/pass.json"));
    let snapshot = recover_public(&events).expect("pass fixture must recover");

    assert_eq!(snapshot.phase, WorkflowPhase::Completed);
    assert_eq!(snapshot.outcome, Some(WorkflowOutcome::Passed));
    assert_eq!(snapshot.repair_attempts, 0);
    assert_eq!(snapshot.event_count, 7);
    assert_eq!(snapshot.evidence_count, 2);
    assert!(snapshot.final_response_present);
}

#[test]
fn repair_then_pass_fixture_consumes_one_bounded_repair() {
    let events = fixture(include_str!("../../../examples/repair_then_pass.json"));
    let snapshot = recover_public(&events).expect("repair fixture must recover");

    assert_eq!(snapshot.phase, WorkflowPhase::Completed);
    assert_eq!(snapshot.outcome, Some(WorkflowOutcome::Passed));
    assert_eq!(snapshot.repair_attempts, 1);
    assert_eq!(snapshot.event_count, 13);
    assert_eq!(snapshot.evidence_count, 4);
    assert!(snapshot.final_response_present);
}

#[test]
fn disagreement_fixture_escalates_to_the_owner() {
    let events = fixture(include_str!(
        "../../../examples/disagreement_escalation.json"
    ));
    let snapshot = recover_public(&events).expect("escalation fixture must recover");

    assert_eq!(snapshot.phase, WorkflowPhase::Completed);
    assert_eq!(snapshot.outcome, Some(WorkflowOutcome::Escalated));
    assert_eq!(snapshot.repair_attempts, 0);
    assert_eq!(snapshot.event_count, 8);
    assert_eq!(snapshot.evidence_count, 2);
    assert!(snapshot.final_response_present);
}

#[test]
fn recovery_is_deterministic_and_omits_intake_text() {
    let events = fixture(include_str!("../../../examples/pass.json"));
    let first = recover_public(&events).unwrap();
    let second = recover_public(&events).unwrap();

    assert_eq!(first, second);
    let public_json = serde_json::to_string(&first).unwrap();
    assert!(!public_json.contains("Create a deterministic greeting formatter"));
    assert!(!public_json.contains("objective"));
    assert!(!public_json.contains("instructions"));
}

#[test]
fn public_projection_never_exposes_caller_controlled_sentinels() {
    let mut events = fixture(include_str!("../../../examples/pass.json"));
    for event in &mut events {
        event.workflow_id = "WORKFLOW_ID_SENTINEL".into();
    }

    let WorkflowEventKind::EngineerReturned { result } = &mut events[2].kind else {
        panic!("fixture event must be an engineer result");
    };
    result.evidence[0].evidence_id = "IMPLEMENTATION_EVIDENCE_SENTINEL".into();

    let WorkflowEventKind::VerificationRecorded { outcome } = &mut events[3].kind else {
        panic!("fixture event must be verification");
    };
    outcome.evidence[0].evidence_id = "VERIFICATION_EVIDENCE_SENTINEL".into();

    let WorkflowEventKind::ReviewerDecided { packet } = &mut events[4].kind else {
        panic!("fixture event must be review");
    };
    packet.evidence_refs = vec!["VERIFICATION_EVIDENCE_SENTINEL".into()];

    let WorkflowEventKind::AuditorCounterchecked { packet } = &mut events[5].kind else {
        panic!("fixture event must be audit");
    };
    packet.evidence_refs = vec!["VERIFICATION_EVIDENCE_SENTINEL".into()];

    let WorkflowEventKind::CoordinatorFinalized { response } = &mut events[6].kind else {
        panic!("fixture event must be final response");
    };
    response.response_id = "FINAL_RESPONSE_ID_SENTINEL".into();
    response.public_summary = "FINAL_RESPONSE_SUMMARY_SENTINEL".into();
    response.evidence_refs = vec!["VERIFICATION_EVIDENCE_SENTINEL".into()];
    response.completed_at = "FINAL_RESPONSE_TIME_SENTINELT00:00:00Z".into();

    let snapshot = recover_public(&events).expect("sentinel fixture must recover");
    assert_eq!(
        serde_json::to_value(&snapshot).unwrap(),
        serde_json::json!({
            "phase": "completed",
            "event_count": 7,
            "repair_attempts": 0,
            "outcome": "passed",
            "evidence_count": 2,
            "final_response_present": true
        })
    );
    let public_json = serde_json::to_string(&snapshot).unwrap();
    for sentinel in [
        "WORKFLOW_ID_SENTINEL",
        "IMPLEMENTATION_EVIDENCE_SENTINEL",
        "VERIFICATION_EVIDENCE_SENTINEL",
        "FINAL_RESPONSE_ID_SENTINEL",
        "FINAL_RESPONSE_SUMMARY_SENTINEL",
        "FINAL_RESPONSE_TIME_SENTINEL",
    ] {
        assert!(!public_json.contains(sentinel));
    }
}

#[test]
fn a_failed_event_is_transactional() {
    let mut events = fixture(include_str!("../../../examples/pass.json"));
    events[1].sequence = 9;
    let mut reducer = WorkflowReducer::new();
    reducer.apply(&events[0]).unwrap();
    let before = reducer.public_snapshot();

    assert!(matches!(
        reducer.apply(&events[1]),
        Err(WorkflowError::SequenceMismatch {
            expected: 1,
            actual: 9
        })
    ));
    assert_eq!(reducer.public_snapshot(), before);
}

#[test]
fn a_failed_engineer_result_cannot_reach_pass() {
    let mut events = fixture(include_str!("../../../examples/pass.json"));
    let WorkflowEventKind::EngineerReturned { result } = &mut events[2].kind else {
        panic!("fixture event must be an engineer result");
    };
    result.status = RoleResultStatus::Failed;

    assert_eq!(
        recover_public(&events),
        Err(WorkflowError::PassedFailedEngineerResult)
    );
}

#[test]
fn initial_dispatch_rejects_an_arbitrary_input_reference() {
    let mut events = fixture(include_str!("../../../examples/pass.json"));
    let WorkflowEventKind::CoordinatorDispatched { invocation } = &mut events[1].kind else {
        panic!("fixture event must be a dispatch");
    };
    invocation.input_refs = vec!["arbitrary-input".into()];

    assert_eq!(
        recover_public(&events),
        Err(WorkflowError::InputReferenceMismatch)
    );
}

#[test]
fn repair_dispatch_rejects_the_stale_intake_reference() {
    let mut events = fixture(include_str!("../../../examples/repair_then_pass.json"));
    let WorkflowEventKind::CoordinatorDispatched { invocation } = &mut events[7].kind else {
        panic!("fixture event must be the repair dispatch");
    };
    invocation.input_refs = vec!["request-repair".into()];

    assert_eq!(
        recover_public(&events),
        Err(WorkflowError::InputReferenceMismatch)
    );
}

#[test]
fn evidence_ids_cannot_collide_across_workflow_events() {
    let mut events = fixture(include_str!("../../../examples/pass.json"));
    let WorkflowEventKind::VerificationRecorded { outcome } = &mut events[3].kind else {
        panic!("fixture event must be verification");
    };
    outcome.evidence[0].evidence_id = "evidence-pass-implementation".into();

    assert_eq!(
        recover_public(&events),
        Err(WorkflowError::DuplicateEvidenceId {
            field: "verification.evidence"
        })
    );
}

#[test]
fn finding_evidence_must_already_be_known() {
    let mut events = fixture(include_str!(
        "../../../examples/disagreement_escalation.json"
    ));
    let WorkflowEventKind::AuditorCounterchecked { packet } = &mut events[5].kind else {
        panic!("fixture event must be an audit packet");
    };
    packet.findings[0].evidence_refs = vec!["unknown-evidence".into()];

    assert_eq!(
        recover_public(&events),
        Err(WorkflowError::UnknownEvidenceReference {
            field: "audit.findings.evidence_refs"
        })
    );
}

#[test]
fn exhausted_repair_budget_takes_the_owner_escalation_branch() {
    let mut events = fixture(include_str!("../../../examples/repair_then_pass.json"));

    let WorkflowEventKind::IntakeAccepted { budget, .. } = &mut events[0].kind else {
        panic!("fixture event must be intake");
    };
    budget.max_events = 14;

    let WorkflowEventKind::VerificationRecorded { outcome } = &mut events[9].kind else {
        panic!("fixture event must be verification");
    };
    outcome.status = VerificationStatus::Failed;
    outcome.checks[0].status = VerificationStatus::Failed;

    let WorkflowEventKind::ReviewerDecided { packet } = &mut events[10].kind else {
        panic!("fixture event must be review");
    };
    packet.decision = ReviewDecision::Repair;
    packet.findings = vec![Finding {
        code: "second-attempt-failed".into(),
        severity: FindingSeverity::Blocking,
        summary: "The bounded repair did not satisfy the check".into(),
        evidence_refs: vec!["evidence-repair-pass".into()],
    }];

    events[12].kind = WorkflowEventKind::OwnerEscalated {
        decision: Decision {
            contract_version: "1.0".into(),
            decision_id: "decision-repair-exhausted".into(),
            made_by: ovca_types::Role::Owner,
            disposition: DecisionDisposition::Escalate,
            rationale: "The bounded repair budget is exhausted".into(),
            evidence_refs: vec!["evidence-repair-pass".into()],
            made_at: "2026-02-01T00:12:00Z".into(),
        },
    };
    events.push(WorkflowEvent {
        contract_version: "1.0".into(),
        workflow_id: "workflow-repair".into(),
        event_id: "repair-13".into(),
        sequence: 13,
        previous_event_id: Some("repair-12".into()),
        occurred_at: "2026-02-01T00:13:00Z".into(),
        kind: WorkflowEventKind::CoordinatorFinalized {
            response: FinalResponse {
                contract_version: "1.0".into(),
                response_id: "response-repair-exhausted".into(),
                outcome: WorkflowOutcome::Escalated,
                public_summary: "The exhausted bounded repair was escalated".into(),
                evidence_refs: vec!["evidence-repair-pass".into()],
                completed_at: "2026-02-01T00:13:00Z".into(),
            },
        },
    });

    let snapshot = recover_public(&events).expect("exhausted repair must escalate cleanly");
    assert_eq!(snapshot.phase, WorkflowPhase::Completed);
    assert_eq!(snapshot.outcome, Some(WorkflowOutcome::Escalated));
    assert_eq!(snapshot.repair_attempts, 1);
    assert_eq!(snapshot.event_count, 14);
}
