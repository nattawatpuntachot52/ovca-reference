//! Domain-neutral, deterministic contracts for evidence-bound role workflows.
//! Callers supply identifiers, timestamps, and event streams.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use thiserror::Error;

pub const CONTRACT_VERSION: &str = "1.0";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Error)]
#[error("{field}: {message}")]
#[serde(deny_unknown_fields)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

impl ValidationError {
    fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
        }
    }
}

pub trait Validate {
    fn validate(&self) -> Result<(), ValidationError>;
}

fn validate_version(version: &str) -> Result<(), ValidationError> {
    if version == CONTRACT_VERSION {
        Ok(())
    } else {
        Err(ValidationError::new(
            "contract_version",
            format!("expected {CONTRACT_VERSION}"),
        ))
    }
}

fn require_text(field: &str, value: &str) -> Result<(), ValidationError> {
    if value.trim().is_empty() {
        Err(ValidationError::new(field, "must not be empty"))
    } else {
        Ok(())
    }
}

fn require_timestamp(field: &str, value: &str) -> Result<(), ValidationError> {
    require_text(field, value)?;
    if value.contains('T') && value.ends_with('Z') {
        Ok(())
    } else {
        Err(ValidationError::new(
            field,
            "must be a caller-supplied UTC timestamp ending in Z",
        ))
    }
}

fn require_non_empty_strings(field: &str, values: &[String]) -> Result<(), ValidationError> {
    if values.is_empty() {
        return Err(ValidationError::new(
            field,
            "must contain at least one item",
        ));
    }
    require_unique_strings(field, values)
}

fn require_unique_strings(field: &str, values: &[String]) -> Result<(), ValidationError> {
    let mut seen = BTreeSet::new();
    for value in values {
        require_text(field, value)?;
        if !seen.insert(value) {
            return Err(ValidationError::new(field, "must not contain duplicates"));
        }
    }
    Ok(())
}

fn validate_evidence_records(field: &str, evidence: &[Evidence]) -> Result<(), ValidationError> {
    if evidence.is_empty() {
        return Err(ValidationError::new(
            field,
            "must contain at least one evidence record",
        ));
    }
    let mut ids = BTreeSet::new();
    for item in evidence {
        item.validate()?;
        if !ids.insert(&item.evidence_id) {
            return Err(ValidationError::new(
                field,
                "must not contain duplicate evidence ids",
            ));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Owner,
    Coordinator,
    Engineer,
    Reviewer,
    Auditor,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Authority {
    pub contract_version: String,
    pub authority_id: String,
    pub granted_by: Role,
    pub objective: String,
    pub scope: Vec<String>,
    pub constraints: Vec<String>,
}

impl Validate for Authority {
    fn validate(&self) -> Result<(), ValidationError> {
        validate_version(&self.contract_version)?;
        require_text("authority_id", &self.authority_id)?;
        if self.granted_by != Role::Owner {
            return Err(ValidationError::new(
                "granted_by",
                "public workflow authority must originate with the owner",
            ));
        }
        require_text("objective", &self.objective)?;
        require_non_empty_strings("scope", &self.scope)?;
        require_unique_strings("constraints", &self.constraints)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    Input,
    Implementation,
    Verification,
    Review,
    Audit,
    Decision,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    pub contract_version: String,
    pub evidence_id: String,
    pub kind: EvidenceKind,
    pub summary: String,
    pub source: String,
    pub collected_at: String,
}

impl Validate for Evidence {
    fn validate(&self) -> Result<(), ValidationError> {
        validate_version(&self.contract_version)?;
        require_text("evidence_id", &self.evidence_id)?;
        require_text("summary", &self.summary)?;
        require_text("source", &self.source)?;
        require_timestamp("collected_at", &self.collected_at)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionDisposition {
    Pass,
    Repair,
    Escalate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    pub contract_version: String,
    pub decision_id: String,
    pub made_by: Role,
    pub disposition: DecisionDisposition,
    pub rationale: String,
    pub evidence_refs: Vec<String>,
    pub made_at: String,
}

impl Validate for Decision {
    fn validate(&self) -> Result<(), ValidationError> {
        validate_version(&self.contract_version)?;
        require_text("decision_id", &self.decision_id)?;
        require_text("rationale", &self.rationale)?;
        require_non_empty_strings("evidence_refs", &self.evidence_refs)?;
        require_timestamp("made_at", &self.made_at)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoleInvocation {
    pub contract_version: String,
    pub invocation_id: String,
    pub requested_by: Role,
    pub role: Role,
    pub objective: String,
    pub authority_ref: String,
    pub input_refs: Vec<String>,
    pub attempt: u32,
    pub requested_at: String,
}

impl Validate for RoleInvocation {
    fn validate(&self) -> Result<(), ValidationError> {
        validate_version(&self.contract_version)?;
        require_text("invocation_id", &self.invocation_id)?;
        require_text("objective", &self.objective)?;
        require_text("authority_ref", &self.authority_ref)?;
        require_unique_strings("input_refs", &self.input_refs)?;
        if self.attempt == 0 {
            return Err(ValidationError::new("attempt", "must be at least 1"));
        }
        require_timestamp("requested_at", &self.requested_at)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoleResultStatus {
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoleResult {
    pub contract_version: String,
    pub result_id: String,
    pub invocation_ref: String,
    pub role: Role,
    pub status: RoleResultStatus,
    pub summary: String,
    pub evidence: Vec<Evidence>,
    pub completed_at: String,
}

impl Validate for RoleResult {
    fn validate(&self) -> Result<(), ValidationError> {
        validate_version(&self.contract_version)?;
        require_text("result_id", &self.result_id)?;
        require_text("invocation_ref", &self.invocation_ref)?;
        require_text("summary", &self.summary)?;
        validate_evidence_records("evidence", &self.evidence)?;
        require_timestamp("completed_at", &self.completed_at)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStatus {
    Passed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationCheck {
    pub name: String,
    pub status: VerificationStatus,
    pub summary: String,
}

impl Validate for VerificationCheck {
    fn validate(&self) -> Result<(), ValidationError> {
        require_text("checks.name", &self.name)?;
        require_text("checks.summary", &self.summary)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationOutcome {
    pub contract_version: String,
    pub verification_id: String,
    pub subject_result_ref: String,
    pub status: VerificationStatus,
    pub checks: Vec<VerificationCheck>,
    pub evidence: Vec<Evidence>,
    pub completed_at: String,
}

impl Validate for VerificationOutcome {
    fn validate(&self) -> Result<(), ValidationError> {
        validate_version(&self.contract_version)?;
        require_text("verification_id", &self.verification_id)?;
        require_text("subject_result_ref", &self.subject_result_ref)?;
        if self.checks.is_empty() {
            return Err(ValidationError::new(
                "checks",
                "must contain at least one check",
            ));
        }
        for check in &self.checks {
            check.validate()?;
        }
        let any_failed = self
            .checks
            .iter()
            .any(|check| check.status == VerificationStatus::Failed);
        if (self.status == VerificationStatus::Passed && any_failed)
            || (self.status == VerificationStatus::Failed && !any_failed)
        {
            return Err(ValidationError::new(
                "status",
                "must agree with the individual check statuses",
            ));
        }
        validate_evidence_records("evidence", &self.evidence)?;
        require_timestamp("completed_at", &self.completed_at)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingSeverity {
    Advisory,
    Blocking,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Finding {
    pub code: String,
    pub severity: FindingSeverity,
    pub summary: String,
    pub evidence_refs: Vec<String>,
}

impl Validate for Finding {
    fn validate(&self) -> Result<(), ValidationError> {
        require_text("findings.code", &self.code)?;
        require_text("findings.summary", &self.summary)?;
        require_non_empty_strings("findings.evidence_refs", &self.evidence_refs)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewDecision {
    Pass,
    Repair,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewPacket {
    pub contract_version: String,
    pub review_id: String,
    pub engineer_result_ref: String,
    pub verification_ref: String,
    pub decision: ReviewDecision,
    pub findings: Vec<Finding>,
    pub evidence_refs: Vec<String>,
    pub completed_at: String,
}

impl Validate for ReviewPacket {
    fn validate(&self) -> Result<(), ValidationError> {
        validate_version(&self.contract_version)?;
        require_text("review_id", &self.review_id)?;
        require_text("engineer_result_ref", &self.engineer_result_ref)?;
        require_text("verification_ref", &self.verification_ref)?;
        for finding in &self.findings {
            finding.validate()?;
        }
        match self.decision {
            ReviewDecision::Repair
                if !self
                    .findings
                    .iter()
                    .any(|finding| finding.severity == FindingSeverity::Blocking) =>
            {
                return Err(ValidationError::new(
                    "findings",
                    "repair decisions require at least one blocking finding",
                ));
            }
            ReviewDecision::Pass
                if self
                    .findings
                    .iter()
                    .any(|finding| finding.severity == FindingSeverity::Blocking) =>
            {
                return Err(ValidationError::new(
                    "decision",
                    "pass decisions cannot contain blocking findings",
                ));
            }
            _ => {}
        }
        require_non_empty_strings("evidence_refs", &self.evidence_refs)?;
        require_timestamp("completed_at", &self.completed_at)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditDecision {
    Concur,
    Disagree,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditPacket {
    pub contract_version: String,
    pub audit_id: String,
    pub review_ref: String,
    pub decision: AuditDecision,
    pub findings: Vec<Finding>,
    pub evidence_refs: Vec<String>,
    pub completed_at: String,
}

impl Validate for AuditPacket {
    fn validate(&self) -> Result<(), ValidationError> {
        validate_version(&self.contract_version)?;
        require_text("audit_id", &self.audit_id)?;
        require_text("review_ref", &self.review_ref)?;
        for finding in &self.findings {
            finding.validate()?;
        }
        if self.decision == AuditDecision::Disagree && self.findings.is_empty() {
            return Err(ValidationError::new(
                "findings",
                "disagreement requires at least one finding",
            ));
        }
        if self.decision == AuditDecision::Concur
            && self
                .findings
                .iter()
                .any(|finding| finding.severity == FindingSeverity::Blocking)
        {
            return Err(ValidationError::new(
                "decision",
                "concurrence cannot contain blocking audit findings",
            ));
        }
        require_non_empty_strings("evidence_refs", &self.evidence_refs)?;
        require_timestamp("completed_at", &self.completed_at)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepairDirective {
    pub contract_version: String,
    pub directive_id: String,
    pub source_review_ref: String,
    pub source_audit_ref: String,
    pub attempt: u32,
    pub instructions: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub issued_at: String,
}

impl Validate for RepairDirective {
    fn validate(&self) -> Result<(), ValidationError> {
        validate_version(&self.contract_version)?;
        require_text("directive_id", &self.directive_id)?;
        require_text("source_review_ref", &self.source_review_ref)?;
        require_text("source_audit_ref", &self.source_audit_ref)?;
        if self.attempt == 0 {
            return Err(ValidationError::new("attempt", "must be at least 1"));
        }
        require_non_empty_strings("instructions", &self.instructions)?;
        require_non_empty_strings("evidence_refs", &self.evidence_refs)?;
        require_timestamp("issued_at", &self.issued_at)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionBudget {
    pub contract_version: String,
    pub max_repair_attempts: u32,
    pub max_events: u32,
}

impl Validate for ExecutionBudget {
    fn validate(&self) -> Result<(), ValidationError> {
        validate_version(&self.contract_version)?;
        if self.max_events == 0 {
            return Err(ValidationError::new("max_events", "must be at least 1"));
        }
        let minimum_events = self
            .max_repair_attempts
            .checked_mul(6)
            .and_then(|repair_events| repair_events.checked_add(8))
            .ok_or_else(|| ValidationError::new("max_repair_attempts", "is too large"))?;
        if self.max_events < minimum_events {
            return Err(ValidationError::new(
                "max_events",
                format!("must be at least {minimum_events} for the declared repair budget"),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowOutcome {
    Passed,
    Escalated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FinalResponse {
    pub contract_version: String,
    pub response_id: String,
    pub outcome: WorkflowOutcome,
    pub public_summary: String,
    pub evidence_refs: Vec<String>,
    pub completed_at: String,
}

impl Validate for FinalResponse {
    fn validate(&self) -> Result<(), ValidationError> {
        validate_version(&self.contract_version)?;
        require_text("response_id", &self.response_id)?;
        require_text("public_summary", &self.public_summary)?;
        require_non_empty_strings("evidence_refs", &self.evidence_refs)?;
        require_timestamp("completed_at", &self.completed_at)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum WorkflowEventKind {
    IntakeAccepted {
        request_ref: String,
        public_summary: String,
        authority: Authority,
        budget: ExecutionBudget,
    },
    CoordinatorDispatched {
        invocation: RoleInvocation,
    },
    EngineerReturned {
        result: RoleResult,
    },
    VerificationRecorded {
        outcome: VerificationOutcome,
    },
    ReviewerDecided {
        packet: ReviewPacket,
    },
    AuditorCounterchecked {
        packet: AuditPacket,
    },
    RepairDirected {
        directive: RepairDirective,
    },
    OwnerEscalated {
        decision: Decision,
    },
    CoordinatorFinalized {
        response: FinalResponse,
    },
}

impl Validate for WorkflowEventKind {
    fn validate(&self) -> Result<(), ValidationError> {
        match self {
            Self::IntakeAccepted {
                request_ref,
                public_summary,
                authority,
                budget,
            } => {
                require_text("request_ref", request_ref)?;
                require_text("public_summary", public_summary)?;
                authority.validate()?;
                budget.validate()
            }
            Self::CoordinatorDispatched { invocation } => invocation.validate(),
            Self::EngineerReturned { result } => result.validate(),
            Self::VerificationRecorded { outcome } => outcome.validate(),
            Self::ReviewerDecided { packet } => packet.validate(),
            Self::AuditorCounterchecked { packet } => packet.validate(),
            Self::RepairDirected { directive } => directive.validate(),
            Self::OwnerEscalated { decision } => {
                decision.validate()?;
                if decision.made_by != Role::Owner
                    || decision.disposition != DecisionDisposition::Escalate
                {
                    return Err(ValidationError::new(
                        "decision",
                        "owner escalation requires an owner escalate decision",
                    ));
                }
                Ok(())
            }
            Self::CoordinatorFinalized { response } => response.validate(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowEvent {
    pub contract_version: String,
    pub workflow_id: String,
    pub event_id: String,
    pub sequence: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_event_id: Option<String>,
    pub occurred_at: String,
    #[serde(flatten)]
    pub kind: WorkflowEventKind,
}

impl Validate for WorkflowEvent {
    fn validate(&self) -> Result<(), ValidationError> {
        validate_version(&self.contract_version)?;
        require_text("workflow_id", &self.workflow_id)?;
        require_text("event_id", &self.event_id)?;
        require_timestamp("occurred_at", &self.occurred_at)?;
        match (self.sequence, &self.previous_event_id) {
            (0, None) => {}
            (0, Some(_)) => {
                return Err(ValidationError::new(
                    "previous_event_id",
                    "the first event must not have a previous event",
                ));
            }
            (_, Some(previous)) => require_text("previous_event_id", previous)?,
            (_, None) => {
                return Err(ValidationError::new(
                    "previous_event_id",
                    "non-initial events require the previous event id",
                ));
            }
        }
        self.kind.validate()
    }
}
