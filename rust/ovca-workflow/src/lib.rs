//! A synchronous, deterministic reference reducer for caller-supplied OVCA events.

use ovca_types::{
    AuditDecision, DecisionDisposition, Evidence, FinalResponse, ReviewDecision, Role,
    RoleResultStatus, Validate, ValidationError, VerificationStatus, WorkflowEvent,
    WorkflowEventKind, WorkflowOutcome,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowPhase {
    AwaitingIntake,
    AwaitingCoordinatorDispatch,
    AwaitingEngineerResult,
    AwaitingVerification,
    AwaitingReview,
    AwaitingAudit,
    AwaitingRepairDirective,
    AwaitingOwnerEscalation,
    AwaitingCoordinatorFinalResponse,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicWorkflowSnapshot {
    pub phase: WorkflowPhase,
    pub event_count: u64,
    pub repair_attempts: u32,
    pub outcome: Option<WorkflowOutcome>,
    pub evidence_count: usize,
    pub final_response_present: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum WorkflowError {
    #[error("invalid event: {0}")]
    InvalidEvent(#[from] ValidationError),
    #[error("unexpected {event_type} event while phase is {phase:?}")]
    UnexpectedEvent {
        phase: WorkflowPhase,
        event_type: &'static str,
    },
    #[error("workflow id mismatch")]
    WorkflowIdMismatch,
    #[error("event sequence mismatch: expected {expected}, got {actual}")]
    SequenceMismatch { expected: u64, actual: u64 },
    #[error("event chain mismatch")]
    PreviousEventMismatch,
    #[error("duplicate event id")]
    DuplicateEventId,
    #[error("event budget exceeded: maximum {maximum}")]
    EventBudgetExceeded { maximum: u32 },
    #[error("reference mismatch for {field}")]
    ReferenceMismatch { field: &'static str },
    #[error("dispatch input reference does not match the current workflow input")]
    InputReferenceMismatch,
    #[error("unknown evidence reference in {field}")]
    UnknownEvidenceReference { field: &'static str },
    #[error("duplicate evidence id introduced by {field}")]
    DuplicateEvidenceId { field: &'static str },
    #[error("role invariant failed for {field}: expected {expected:?}, got {actual:?}")]
    RoleMismatch {
        field: &'static str,
        expected: Role,
        actual: Role,
    },
    #[error("attempt mismatch: expected {expected}, got {actual}")]
    AttemptMismatch { expected: u32, actual: u32 },
    #[error("review cannot pass a failed verification")]
    PassedFailedVerification,
    #[error("review cannot pass a failed engineer result")]
    PassedFailedEngineerResult,
    #[error("owner escalation must use the escalate disposition")]
    InvalidOwnerEscalation,
    #[error("final response outcome mismatch: expected {expected:?}, got {actual:?}")]
    OutcomeMismatch {
        expected: WorkflowOutcome,
        actual: WorkflowOutcome,
    },
    #[error("workflow state is missing {0}")]
    MissingState(&'static str),
}

#[derive(Debug, Clone)]
pub struct WorkflowReducer {
    workflow_id: Option<String>,
    phase: WorkflowPhase,
    event_count: u64,
    last_event_id: Option<String>,
    seen_event_ids: BTreeSet<String>,
    authority_id: Option<String>,
    expected_input_ref: Option<String>,
    max_repair_attempts: Option<u32>,
    max_events: Option<u32>,
    repair_attempts: u32,
    current_attempt: u32,
    invocation_id: Option<String>,
    result_id: Option<String>,
    result_status: Option<RoleResultStatus>,
    verification_id: Option<String>,
    verification_status: Option<VerificationStatus>,
    review_id: Option<String>,
    review_decision: Option<ReviewDecision>,
    audit_id: Option<String>,
    pending_outcome: Option<WorkflowOutcome>,
    final_response: Option<FinalResponse>,
    evidence_ids: BTreeSet<String>,
}

impl Default for WorkflowReducer {
    fn default() -> Self {
        Self {
            workflow_id: None,
            phase: WorkflowPhase::AwaitingIntake,
            event_count: 0,
            last_event_id: None,
            seen_event_ids: BTreeSet::new(),
            authority_id: None,
            expected_input_ref: None,
            max_repair_attempts: None,
            max_events: None,
            repair_attempts: 0,
            current_attempt: 1,
            invocation_id: None,
            result_id: None,
            result_status: None,
            verification_id: None,
            verification_status: None,
            review_id: None,
            review_decision: None,
            audit_id: None,
            pending_outcome: None,
            final_response: None,
            evidence_ids: BTreeSet::new(),
        }
    }
}

impl WorkflowReducer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply(&mut self, event: &WorkflowEvent) -> Result<(), WorkflowError> {
        let mut next = self.clone();
        next.apply_inner(event)?;
        *self = next;
        Ok(())
    }

    pub fn public_snapshot(&self) -> PublicWorkflowSnapshot {
        PublicWorkflowSnapshot {
            phase: self.phase,
            event_count: self.event_count,
            repair_attempts: self.repair_attempts,
            outcome: self
                .final_response
                .as_ref()
                .map(|response| response.outcome)
                .or(self.pending_outcome),
            evidence_count: self.evidence_ids.len(),
            final_response_present: self.final_response.is_some(),
        }
    }

    fn apply_inner(&mut self, event: &WorkflowEvent) -> Result<(), WorkflowError> {
        event.validate()?;
        self.validate_envelope(event)?;

        match &event.kind {
            WorkflowEventKind::IntakeAccepted {
                request_ref,
                authority,
                budget,
                ..
            } => {
                self.expect_phase(WorkflowPhase::AwaitingIntake, "intake_accepted")?;
                self.workflow_id = Some(event.workflow_id.clone());
                self.authority_id = Some(authority.authority_id.clone());
                self.expected_input_ref = Some(request_ref.clone());
                self.max_repair_attempts = Some(budget.max_repair_attempts);
                self.max_events = Some(budget.max_events);
                self.phase = WorkflowPhase::AwaitingCoordinatorDispatch;
            }
            WorkflowEventKind::CoordinatorDispatched { invocation } => {
                self.expect_phase(
                    WorkflowPhase::AwaitingCoordinatorDispatch,
                    "coordinator_dispatched",
                )?;
                self.expect_role("requested_by", Role::Coordinator, invocation.requested_by)?;
                self.expect_role("role", Role::Engineer, invocation.role)?;
                self.expect_ref(
                    "authority_ref",
                    self.authority_id.as_deref(),
                    &invocation.authority_ref,
                )?;
                if invocation.attempt != self.current_attempt {
                    return Err(WorkflowError::AttemptMismatch {
                        expected: self.current_attempt,
                        actual: invocation.attempt,
                    });
                }
                self.expect_input_ref(&invocation.input_refs)?;
                self.invocation_id = Some(invocation.invocation_id.clone());
                self.expected_input_ref = None;
                self.phase = WorkflowPhase::AwaitingEngineerResult;
            }
            WorkflowEventKind::EngineerReturned { result } => {
                self.expect_phase(WorkflowPhase::AwaitingEngineerResult, "engineer_returned")?;
                self.expect_role("role", Role::Engineer, result.role)?;
                self.expect_ref(
                    "invocation_ref",
                    self.invocation_id.as_deref(),
                    &result.invocation_ref,
                )?;
                self.result_id = Some(result.result_id.clone());
                self.result_status = Some(result.status);
                self.register_evidence("engineer.evidence", &result.evidence)?;
                self.phase = WorkflowPhase::AwaitingVerification;
            }
            WorkflowEventKind::VerificationRecorded { outcome } => {
                self.expect_phase(WorkflowPhase::AwaitingVerification, "verification_recorded")?;
                self.expect_ref(
                    "subject_result_ref",
                    self.result_id.as_deref(),
                    &outcome.subject_result_ref,
                )?;
                self.verification_id = Some(outcome.verification_id.clone());
                self.verification_status = Some(outcome.status);
                self.register_evidence("verification.evidence", &outcome.evidence)?;
                self.phase = WorkflowPhase::AwaitingReview;
            }
            WorkflowEventKind::ReviewerDecided { packet } => {
                self.expect_phase(WorkflowPhase::AwaitingReview, "reviewer_decided")?;
                self.expect_ref(
                    "engineer_result_ref",
                    self.result_id.as_deref(),
                    &packet.engineer_result_ref,
                )?;
                self.expect_ref(
                    "verification_ref",
                    self.verification_id.as_deref(),
                    &packet.verification_ref,
                )?;
                if packet.decision == ReviewDecision::Pass
                    && self.verification_status == Some(VerificationStatus::Failed)
                {
                    return Err(WorkflowError::PassedFailedVerification);
                }
                if packet.decision == ReviewDecision::Pass
                    && self.result_status != Some(RoleResultStatus::Succeeded)
                {
                    return Err(WorkflowError::PassedFailedEngineerResult);
                }
                self.review_id = Some(packet.review_id.clone());
                self.review_decision = Some(packet.decision);
                self.expect_known_evidence("review.evidence_refs", &packet.evidence_refs)?;
                for finding in &packet.findings {
                    self.expect_known_evidence(
                        "review.findings.evidence_refs",
                        &finding.evidence_refs,
                    )?;
                }
                self.phase = WorkflowPhase::AwaitingAudit;
            }
            WorkflowEventKind::AuditorCounterchecked { packet } => {
                self.expect_phase(WorkflowPhase::AwaitingAudit, "auditor_counterchecked")?;
                self.expect_ref("review_ref", self.review_id.as_deref(), &packet.review_ref)?;
                self.audit_id = Some(packet.audit_id.clone());
                self.expect_known_evidence("audit.evidence_refs", &packet.evidence_refs)?;
                for finding in &packet.findings {
                    self.expect_known_evidence(
                        "audit.findings.evidence_refs",
                        &finding.evidence_refs,
                    )?;
                }
                if packet.decision == AuditDecision::Disagree {
                    self.phase = WorkflowPhase::AwaitingOwnerEscalation;
                } else {
                    match self
                        .review_decision
                        .ok_or(WorkflowError::MissingState("review decision"))?
                    {
                        ReviewDecision::Pass => {
                            self.pending_outcome = Some(WorkflowOutcome::Passed);
                            self.phase = WorkflowPhase::AwaitingCoordinatorFinalResponse;
                        }
                        ReviewDecision::Repair => {
                            let maximum = self
                                .max_repair_attempts
                                .ok_or(WorkflowError::MissingState("repair budget"))?;
                            self.phase = if self.repair_attempts < maximum {
                                WorkflowPhase::AwaitingRepairDirective
                            } else {
                                WorkflowPhase::AwaitingOwnerEscalation
                            };
                        }
                    }
                }
            }
            WorkflowEventKind::RepairDirected { directive } => {
                self.expect_phase(WorkflowPhase::AwaitingRepairDirective, "repair_directed")?;
                self.expect_ref(
                    "source_review_ref",
                    self.review_id.as_deref(),
                    &directive.source_review_ref,
                )?;
                self.expect_ref(
                    "source_audit_ref",
                    self.audit_id.as_deref(),
                    &directive.source_audit_ref,
                )?;
                if directive.attempt != self.current_attempt {
                    return Err(WorkflowError::AttemptMismatch {
                        expected: self.current_attempt,
                        actual: directive.attempt,
                    });
                }
                self.expect_known_evidence("repair.evidence_refs", &directive.evidence_refs)?;
                self.repair_attempts += 1;
                self.current_attempt += 1;
                self.expected_input_ref = Some(directive.directive_id.clone());
                self.pending_outcome = None;
                self.phase = WorkflowPhase::AwaitingCoordinatorDispatch;
            }
            WorkflowEventKind::OwnerEscalated { decision } => {
                self.expect_phase(WorkflowPhase::AwaitingOwnerEscalation, "owner_escalated")?;
                if decision.made_by != Role::Owner
                    || decision.disposition != DecisionDisposition::Escalate
                {
                    return Err(WorkflowError::InvalidOwnerEscalation);
                }
                self.expect_known_evidence("decision.evidence_refs", &decision.evidence_refs)?;
                self.pending_outcome = Some(WorkflowOutcome::Escalated);
                self.phase = WorkflowPhase::AwaitingCoordinatorFinalResponse;
            }
            WorkflowEventKind::CoordinatorFinalized { response } => {
                self.expect_phase(
                    WorkflowPhase::AwaitingCoordinatorFinalResponse,
                    "coordinator_finalized",
                )?;
                let expected = self
                    .pending_outcome
                    .ok_or(WorkflowError::MissingState("pending outcome"))?;
                if response.outcome != expected {
                    return Err(WorkflowError::OutcomeMismatch {
                        expected,
                        actual: response.outcome,
                    });
                }
                self.expect_known_evidence("response.evidence_refs", &response.evidence_refs)?;
                self.final_response = Some(response.clone());
                self.phase = WorkflowPhase::Completed;
            }
        }

        self.seen_event_ids.insert(event.event_id.clone());
        self.last_event_id = Some(event.event_id.clone());
        self.event_count += 1;
        Ok(())
    }

    fn validate_envelope(&self, event: &WorkflowEvent) -> Result<(), WorkflowError> {
        if let Some(workflow_id) = &self.workflow_id {
            if event.workflow_id != *workflow_id {
                return Err(WorkflowError::WorkflowIdMismatch);
            }
        }
        if event.sequence != self.event_count {
            return Err(WorkflowError::SequenceMismatch {
                expected: self.event_count,
                actual: event.sequence,
            });
        }
        if event.previous_event_id != self.last_event_id {
            return Err(WorkflowError::PreviousEventMismatch);
        }
        if self.seen_event_ids.contains(&event.event_id) {
            return Err(WorkflowError::DuplicateEventId);
        }
        if let Some(maximum) = self.max_events {
            if self.event_count >= u64::from(maximum) {
                return Err(WorkflowError::EventBudgetExceeded { maximum });
            }
        }
        Ok(())
    }

    fn expect_phase(
        &self,
        expected: WorkflowPhase,
        event_type: &'static str,
    ) -> Result<(), WorkflowError> {
        if self.phase == expected {
            Ok(())
        } else {
            Err(WorkflowError::UnexpectedEvent {
                phase: self.phase,
                event_type,
            })
        }
    }

    fn expect_role(
        &self,
        field: &'static str,
        expected: Role,
        actual: Role,
    ) -> Result<(), WorkflowError> {
        if actual == expected {
            Ok(())
        } else {
            Err(WorkflowError::RoleMismatch {
                field,
                expected,
                actual,
            })
        }
    }

    fn expect_ref(
        &self,
        field: &'static str,
        expected: Option<&str>,
        actual: &str,
    ) -> Result<(), WorkflowError> {
        let expected = expected.ok_or(WorkflowError::MissingState(field))?;
        if actual == expected {
            Ok(())
        } else {
            Err(WorkflowError::ReferenceMismatch { field })
        }
    }

    fn expect_known_evidence(
        &self,
        field: &'static str,
        refs: &[String],
    ) -> Result<(), WorkflowError> {
        if refs.iter().all(|item| self.evidence_ids.contains(item)) {
            Ok(())
        } else {
            Err(WorkflowError::UnknownEvidenceReference { field })
        }
    }

    fn expect_input_ref(&self, refs: &[String]) -> Result<(), WorkflowError> {
        let expected = self
            .expected_input_ref
            .as_deref()
            .ok_or(WorkflowError::MissingState("dispatch input reference"))?;
        if refs.len() == 1 && refs[0] == expected {
            Ok(())
        } else {
            Err(WorkflowError::InputReferenceMismatch)
        }
    }

    fn register_evidence(
        &mut self,
        field: &'static str,
        evidence: &[Evidence],
    ) -> Result<(), WorkflowError> {
        for record in evidence {
            if !self.evidence_ids.insert(record.evidence_id.clone()) {
                return Err(WorkflowError::DuplicateEvidenceId { field });
            }
        }
        Ok(())
    }
}

pub fn recover_public(events: &[WorkflowEvent]) -> Result<PublicWorkflowSnapshot, WorkflowError> {
    let mut reducer = WorkflowReducer::new();
    for event in events {
        reducer.apply(event)?;
    }
    Ok(reducer.public_snapshot())
}
