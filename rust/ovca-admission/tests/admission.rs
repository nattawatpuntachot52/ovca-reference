use ovca_admission::{
    evaluate_followup, evaluate_proactive_admission, intersect_rule_scopes, is_path_within_scope,
    revalidate_mandate, sha256_hex, AcceptMode, AddressedKind, AdmissionDecisionKind,
    AdmissionDecisionRecord, AdmissionPolicy, AssessmentInput, AuthorizationMandate,
    FollowUpAction, FollowUpEvent, MandateStatus, MandateStrength, ProactiveDecisionMeta,
    ProactiveRule, ProactiveTrigger, ReceivingScope, RuleMatchCandidate, SubscriptionMode,
    TriggerSourceKind,
};
use ovca_types::{Authority, Role, Validate};
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

fn sample_policy() -> AdmissionPolicy {
    serde_json::from_str(include_str!(
        "../../../contracts/samples/admission_policy.v1.sample.json"
    ))
    .expect("sample policy must deserialize")
}

#[test]
fn admission_policy_schema_and_sample_are_semantically_valid() {
    assert_contract::<AdmissionPolicy>(
        include_str!("../../../contracts/admission_policy.v1.schema.json"),
        include_str!("../../../contracts/samples/admission_policy.v1.sample.json"),
    );
}

#[test]
fn authorization_mandate_schema_and_sample_are_semantically_valid() {
    assert_contract::<AuthorizationMandate>(
        include_str!("../../../contracts/authorization_mandate.v1.schema.json"),
        include_str!("../../../contracts/samples/authorization_mandate.v1.sample.json"),
    );
}

#[test]
fn admission_decision_schema_and_sample_are_semantically_valid() {
    assert_contract::<AdmissionDecisionRecord>(
        include_str!("../../../contracts/admission_decision.v1.schema.json"),
        include_str!("../../../contracts/samples/admission_decision.v1.sample.json"),
    );
}

#[test]
fn sha256_matches_nist_vectors_and_policy_version_is_order_invariant() {
    assert_eq!(
        sha256_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );

    let mut policy_a = sample_policy();
    policy_a.allowed_actors = vec!["Beta-Owner".into(), "alpha-owner".into()];
    policy_a.proactive_rules.push(ProactiveRule {
        rule_id: "aardvark-rule".into(),
        enabled: true,
        title: "Secondary check".into(),
        description: "Test rule ordering invariance.".into(),
        receiving_scopes: vec![ReceivingScope::GithubPr, ReceivingScope::GithubIssue],
        writable_scope: vec!["docs/".into(), "contracts".into()],
        silent_ack: false,
    });

    let mut policy_b = policy_a.clone();
    policy_b.allowed_actors.reverse();
    policy_b.proactive_rules.reverse();
    policy_b.proactive_rules[0].receiving_scopes.reverse();
    policy_b.proactive_rules[0].writable_scope.reverse();

    assert_eq!(
        policy_a.canonical_policy_version().unwrap(),
        policy_b.canonical_policy_version().unwrap()
    );
}

#[test]
fn segment_safe_scope_containment_blocks_prefix_confusion_and_traversal() {
    let allowed = vec!["rust/ovca-types".to_string(), "docs".to_string()];
    assert!(is_path_within_scope("rust/ovca-types/src/lib.rs", &allowed));
    assert!(is_path_within_scope("docs/architecture.md", &allowed));
    assert!(!is_path_within_scope(
        "rust/ovca-types-extra/src/lib.rs",
        &allowed
    ));
    assert!(!is_path_within_scope(
        "rust/ovca-types/../ovca-workflow/src/lib.rs",
        &allowed
    ));
    assert!(!is_path_within_scope(
        "/rust/ovca-types/src/lib.rs",
        &allowed
    ));
    assert!(!is_path_within_scope(
        "C:/rust/ovca-types/src/lib.rs",
        &allowed
    ));
    assert!(!is_path_within_scope(
        "rust\\ovca-types\\src\\lib.rs",
        &allowed
    ));
}

#[test]
fn mandate_binds_to_v1_authority_and_rejects_scope_widening_or_bot_explicit() {
    let authority = Authority {
        contract_version: "1.0".into(),
        authority_id: "authority-example-1".into(),
        granted_by: Role::Owner,
        objective: "Bound pre-intake mandate to owner authority".into(),
        scope: vec!["rust".into(), "contracts".into()],
        constraints: vec!["no network access".into()],
    };

    let mandate: AuthorizationMandate = serde_json::from_str(include_str!(
        "../../../contracts/samples/authorization_mandate.v1.sample.json"
    ))
    .unwrap();
    mandate
        .bind_to_authority(&authority)
        .expect("sub-path mandate should bind cleanly to authority");

    let mut widened = mandate.clone();
    widened.writable_scope.push("skills/zoom-out".into());
    assert!(widened.bind_to_authority(&authority).is_err());

    let mut bot_explicit = mandate;
    bot_explicit.requester_actor = "helper[bot]".into();
    bot_explicit.strength = MandateStrength::Explicit;
    assert!(bot_explicit.validate().is_err());
}

#[test]
fn followup_enforces_context_is_not_authority_and_emits_handback_on_scope_expansion() {
    let policy = sample_policy();
    let policy_version = policy.canonical_policy_version().unwrap();
    let mandate = AuthorizationMandate {
        contract_version: "1.0".into(),
        mandate_id: "mandate-pr-1".into(),
        authority_id: "authority-example-1".into(),
        origin_thread_id: "thread-pr-1".into(),
        source_kind: TriggerSourceKind::GithubPr,
        receiving_scope: ReceivingScope::GithubPr,
        requester_actor: "maintainer-example".into(),
        requester_is_bot: false,
        strength: MandateStrength::Explicit,
        policy_version,
        matched_rules: vec![],
        writable_scope: vec!["rust/ovca-admission".into()],
        status: MandateStatus::Ended,
        issued_at: "2026-01-15T12:00:00Z".into(),
    };

    let bot_ping = FollowUpEvent {
        trigger_id: "trig-bot-1".into(),
        thread_id: "thread-pr-1".into(),
        subscription_mode: SubscriptionMode::OwnedWork,
        actor: "ci-audit-example[bot]".into(),
        actor_is_bot: true,
        is_ci: false,
        addressed: AddressedKind::Mention,
        requested_scope_paths: vec!["rust/ovca-admission/src/lib.rs".into()],
        requests_stop: true,
        requests_scope_expansion: false,
    };
    let bot_decision = evaluate_followup(&mandate, &bot_ping, &policy).unwrap();
    assert_eq!(bot_decision.action, FollowUpAction::ContextOnlyDeliver);
    assert!(!bot_decision.grant_authority);
    assert!(!bot_decision.allow_resume);

    let out_of_scope_human = FollowUpEvent {
        trigger_id: "trig-human-2".into(),
        thread_id: "thread-pr-1".into(),
        subscription_mode: SubscriptionMode::OwnedWork,
        actor: "maintainer-example".into(),
        actor_is_bot: false,
        is_ci: false,
        addressed: AddressedKind::Mention,
        requested_scope_paths: vec!["rust/ovca-workflow/src/lib.rs".into()],
        requests_stop: false,
        requests_scope_expansion: false,
    };
    let handback_decision = evaluate_followup(&mandate, &out_of_scope_human, &policy).unwrap();
    assert_eq!(handback_decision.action, FollowUpAction::HandbackRequired);
    assert!(!handback_decision.grant_authority);
    let handback = handback_decision
        .handback
        .expect("handback record must be emitted");
    assert_eq!(handback.route_to, "fresh_eval");
    assert_eq!(
        handback.out_of_scope_paths,
        vec!["rust/ovca-workflow/src/lib.rs"]
    );

    let in_scope_human = FollowUpEvent {
        requested_scope_paths: vec!["rust/ovca-admission/src/lib.rs".into()],
        ..out_of_scope_human
    };
    let resumed = evaluate_followup(&mandate, &in_scope_human, &policy).unwrap();
    assert_eq!(resumed.action, FollowUpAction::ProceedInScope);
    assert!(resumed.grant_authority);
    assert!(resumed.allow_resume);
}

#[test]
fn observe_mode_records_would_accept_without_mandate_and_keeps_unavailable_distinct() {
    let mut policy = sample_policy();
    let trigger = ProactiveTrigger {
        trigger_id: "trig-ci-9".into(),
        thread_id: "thread-ci-main".into(),
        source_kind: TriggerSourceKind::GithubDefaultBranchCi,
        receiving_scope: ReceivingScope::GithubDefaultBranchCi,
        actor: "github-actions".into(),
        actor_is_bot: false,
        addressed: AddressedKind::None,
    };
    let meta = ProactiveDecisionMeta {
        decision_id: "dec-1".into(),
        mandate_id: Some("mandate-auto-1".into()),
        authority_id: Some("authority-example-1".into()),
        recorded_at: "2026-01-15T12:10:00Z".into(),
    };

    let unavailable = AssessmentInput::Unavailable {
        error_kind: "classifier_timeout".into(),
        reason: "assessment backend timed out before producing rule matches".into(),
    };
    let (unavail_record, unavail_mandate) =
        evaluate_proactive_admission(&policy, &trigger, &unavailable, &meta).unwrap();
    assert_eq!(unavail_record.decision, AdmissionDecisionKind::Unavailable);
    assert!(!unavail_record.would_accept);
    assert_eq!(
        unavail_record.error_kind.as_deref(),
        Some("classifier_timeout")
    );
    assert!(unavail_mandate.is_none());

    let matched = AssessmentInput::Evaluated {
        candidates: vec![RuleMatchCandidate {
            rule_id: "default-branch-ci-regression".into(),
            matched: true,
            reason: "rust job failed on main".into(),
        }],
    };
    let (observe_record, observe_mandate) =
        evaluate_proactive_admission(&policy, &trigger, &matched, &meta).unwrap();
    assert_eq!(observe_record.decision, AdmissionDecisionKind::Ignore);
    assert!(observe_record.would_accept);
    assert!(observe_mandate.is_none());

    policy.accept_mode = AcceptMode::Auto;
    let (auto_record, auto_mandate) =
        evaluate_proactive_admission(&policy, &trigger, &matched, &meta).unwrap();
    assert_eq!(auto_record.decision, AdmissionDecisionKind::Accept);
    assert!(auto_record.would_accept);
    let issued = auto_mandate.expect("auto mode must issue bounded inferred mandate");
    assert_eq!(issued.strength, MandateStrength::Inferred);
    assert_eq!(issued.writable_scope, vec!["contracts", "rust"]);

    policy.accept_mode = AcceptMode::Observe;
    let reval = revalidate_mandate(&issued, &policy).unwrap();
    assert!(!reval.valid_to_resume);
    assert_eq!(reval.resulting_status, MandateStatus::Revoked);
}

#[test]
fn intersect_rule_scopes_keeps_only_mutually_contained_subtrees() {
    let r1 = ProactiveRule {
        rule_id: "r1".into(),
        enabled: true,
        title: "Broad rust".into(),
        description: "Broad rust scope.".into(),
        receiving_scopes: vec![ReceivingScope::GithubPr],
        writable_scope: vec!["rust".into(), "docs".into()],
        silent_ack: true,
    };
    let r2 = ProactiveRule {
        rule_id: "r2".into(),
        enabled: true,
        title: "Narrow admission".into(),
        description: "Narrow admission scope.".into(),
        receiving_scopes: vec![ReceivingScope::GithubPr],
        writable_scope: vec!["rust/ovca-admission".into()],
        silent_ack: true,
    };
    assert_eq!(
        intersect_rule_scopes(&[&r1, &r2]),
        vec!["rust/ovca-admission"]
    );
}

#[test]
fn schema_required_collections_and_unknown_fields_are_enforced_on_the_wire() {
    fn assert_missing_rejected<T: DeserializeOwned>(sample: &str, field: &str) {
        let mut val: serde_json::Value = serde_json::from_str(sample).unwrap();
        val.as_object_mut().unwrap().remove(field).unwrap();
        assert!(serde_json::from_value::<T>(val).is_err());
    }

    let policy_sample = include_str!("../../../contracts/samples/admission_policy.v1.sample.json");
    assert_missing_rejected::<AdmissionPolicy>(policy_sample, "bot_allowlist");
    assert_missing_rejected::<AdmissionPolicy>(policy_sample, "automated_reviewers_non_bot");
    assert_missing_rejected::<AdmissionPolicy>(policy_sample, "proactive_rules");

    let mandate_sample =
        include_str!("../../../contracts/samples/authorization_mandate.v1.sample.json");
    assert_missing_rejected::<AuthorizationMandate>(mandate_sample, "matched_rules");
    assert_missing_rejected::<AuthorizationMandate>(mandate_sample, "writable_scope");

    let mut unknown: serde_json::Value = serde_json::from_str(mandate_sample).unwrap();
    unknown
        .as_object_mut()
        .unwrap()
        .insert("unexpected".into(), serde_json::Value::Bool(true));
    assert!(serde_json::from_value::<AuthorizationMandate>(unknown).is_err());
}
