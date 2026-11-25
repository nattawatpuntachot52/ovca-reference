//! Domain-neutral pre-intake admission contracts and deterministic evaluation helpers.
//!
//! Enforces two pre-intake invariants around unchanged `ovca-types` v1.0 workflow records:
//! 1. `Context != Authority`: immutable `AuthorizationMandate` bound to canonical SHA-256
//!    `policy_version` and repository-relative `writable_scope`, revalidated before resume,
//!    with out-of-scope follow-ups returning a structured `HandbackRecord`.
//! 2. Proactive `Observe` calibration (`off | observe | auto`): `observe` evaluates rules and
//!    records `would_accept` without issuing a mandate, while keeping `unavailable` distinct
//!    from `ignore`.

use ovca_types::{Authority, Validate, ValidationError, CONTRACT_VERSION};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AdmissionError {
    #[error(transparent)]
    Validation(#[from] ValidationError),
    #[error("policy error: {0}")]
    Policy(String),
    #[error("mandate error: {0}")]
    Mandate(String),
}

fn validation_err(field: impl Into<String>, message: impl Into<String>) -> ValidationError {
    ValidationError {
        field: field.into(),
        message: message.into(),
    }
}

fn require_version(version: &str) -> Result<(), ValidationError> {
    if version == CONTRACT_VERSION {
        Ok(())
    } else {
        Err(validation_err(
            "contract_version",
            format!("expected {CONTRACT_VERSION}"),
        ))
    }
}

fn require_text(field: &str, value: &str) -> Result<(), ValidationError> {
    if value.trim().is_empty() {
        Err(validation_err(field, "must not be empty"))
    } else {
        Ok(())
    }
}

fn require_timestamp(field: &str, value: &str) -> Result<(), ValidationError> {
    require_text(field, value)?;
    if value.contains('T') && value.ends_with('Z') {
        Ok(())
    } else {
        Err(validation_err(
            field,
            "must be a caller-supplied UTC timestamp ending in Z",
        ))
    }
}

fn require_policy_version(field: &str, value: &str) -> Result<(), ValidationError> {
    let trimmed = value.trim();
    if let Some(hex) = trimmed.strip_prefix("sha256:") {
        if hex.len() == 64 && hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
            return Ok(());
        }
    }
    Err(validation_err(
        field,
        "must match sha256:<64 lowercase hex characters>",
    ))
}

fn normalize_actor(actor: &str) -> String {
    actor.trim().to_lowercase()
}

fn is_bot_login(actor: &str) -> bool {
    normalize_actor(actor).ends_with("[bot]")
}

fn normalize_actor_list(
    field: &str,
    items: &[String],
    require_non_empty: bool,
) -> Result<BTreeSet<String>, ValidationError> {
    if require_non_empty && items.is_empty() {
        return Err(validation_err(field, "must contain at least one actor"));
    }
    let mut set = BTreeSet::new();
    for item in items {
        let norm = normalize_actor(item);
        if norm.is_empty() {
            return Err(validation_err(
                field,
                "must not contain empty actor entries",
            ));
        }
        if !set.insert(norm) {
            return Err(validation_err(field, "must not contain duplicate actors"));
        }
    }
    Ok(set)
}

fn normalized_segments(raw: &str) -> Option<Vec<&str>> {
    let clean = raw.trim().trim_end_matches('/');
    if clean.is_empty()
        || clean.starts_with('/')
        || clean.contains('\\')
        || clean.contains('\0')
        || clean.chars().nth(1) == Some(':')
    {
        return None;
    }
    let mut parts = Vec::new();
    for seg in clean.split('/') {
        if seg.is_empty() || seg == "." || seg == ".." {
            return None;
        }
        parts.push(seg);
    }
    Some(parts)
}

fn validate_safe_scope_list(
    field: &str,
    values: &[String],
    require_non_empty: bool,
) -> Result<(), ValidationError> {
    if require_non_empty && values.is_empty() {
        return Err(validation_err(field, "must contain at least one path"));
    }
    let mut seen = BTreeSet::new();
    for raw in values {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(validation_err(field, "must not contain empty paths"));
        }
        if trimmed.starts_with('/') || trimmed.contains('\\') || trimmed.contains('\0') {
            return Err(validation_err(
                field,
                "paths must be repository-relative with forward slashes",
            ));
        }
        let Some(segments) = normalized_segments(trimmed) else {
            return Err(validation_err(
                field,
                "paths must not contain '.', '..', empty segments, or drive prefixes",
            ));
        };
        let canonical = segments.join("/");
        if !seen.insert(canonical) {
            return Err(validation_err(field, "must not contain duplicate paths"));
        }
    }
    Ok(())
}

fn canonical_scope_entry(raw: &str) -> String {
    match normalized_segments(raw) {
        Some(parts) => parts.join("/"),
        None => raw.trim().to_string(),
    }
}

/// Pure-Rust FIPS 180-4 SHA-256 hex digest with zero external crate dependencies.
pub fn sha256_hex(input: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    let mut state: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    let bit_len = (input.len() as u64) * 8;
    let mut padded = Vec::with_capacity(input.len() + 72);
    padded.extend_from_slice(input);
    padded.push(0x80);
    while (padded.len() % 64) != 56 {
        padded.push(0x00);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in padded.chunks_exact(64) {
        let mut w = [0u32; 64];
        for (i, word) in w.iter_mut().take(16).enumerate() {
            let offset = i * 4;
            *word = u32::from_be_bytes([
                chunk[offset],
                chunk[offset + 1],
                chunk[offset + 2],
                chunk[offset + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = state;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
        state[4] = state[4].wrapping_add(e);
        state[5] = state[5].wrapping_add(f);
        state[6] = state[6].wrapping_add(g);
        state[7] = state[7].wrapping_add(h);
    }

    let mut out = String::with_capacity(64);
    for word in state {
        out.push_str(&format!("{word:08x}"));
    }
    out
}

/// Segment-boundary containment check preventing prefix confusion (`src/core` vs `src/core_extra`),
/// path traversal (`..`), absolute paths, drive prefixes, and backslashes.
pub fn is_path_within_scope(candidate: &str, allowed_scopes: &[String]) -> bool {
    let trimmed = candidate.trim();
    if trimmed.is_empty()
        || trimmed.starts_with('/')
        || trimmed.starts_with('\\')
        || trimmed.contains('\\')
        || trimmed.contains('\0')
        || trimmed.chars().nth(1) == Some(':')
    {
        return false;
    }

    let Some(cand_parts) = normalized_segments(trimmed) else {
        return false;
    };
    if cand_parts.is_empty() {
        return false;
    }

    for raw_scope in allowed_scopes {
        let Some(scope_parts) = normalized_segments(raw_scope) else {
            continue;
        };
        if !scope_parts.is_empty()
            && cand_parts.len() >= scope_parts.len()
            && cand_parts[..scope_parts.len()] == scope_parts
        {
            return true;
        }
    }
    false
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AcceptMode {
    Off,
    Observe,
    Auto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdmissionChoice {
    Explicit,
    Inferred,
    Ignore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnaddressedChoice {
    Inferred,
    Ignore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceivingScope {
    GithubComment,
    GithubDefaultBranchCi,
    GithubIssue,
    GithubPr,
    OwnerCli,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerSourceKind {
    OwnerCli,
    GithubIssue,
    GithubPr,
    GithubComment,
    GithubDefaultBranchCi,
    Bot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MandateStrength {
    Explicit,
    Inferred,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MandateStatus {
    Active,
    Ended,
    StoppedByPerson,
    Revoked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionMode {
    OwnedWork,
    ContextOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AddressedKind {
    Mention,
    Direct,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdmissionTableRow {
    pub receive_all: bool,
    pub listed_addressed: AdmissionChoice,
    pub listed_unaddressed: UnaddressedChoice,
    pub unlisted_addressed: AdmissionChoice,
    pub unlisted_unaddressed: UnaddressedChoice,
}

impl AdmissionTableRow {
    fn validate_for_scope(&self, scope: ReceivingScope) -> Result<(), ValidationError> {
        if scope == ReceivingScope::GithubDefaultBranchCi {
            if self.listed_addressed != AdmissionChoice::Ignore
                || self.listed_unaddressed != UnaddressedChoice::Ignore
                || self.unlisted_addressed != AdmissionChoice::Ignore
            {
                return Err(validation_err(
                    "admission_table.github_default_branch_ci",
                    "default-branch CI has no human sender; only unlisted_unaddressed may be non-ignore",
                ));
            }
        } else if !self.receive_all
            && (self.listed_unaddressed != UnaddressedChoice::Ignore
                || self.unlisted_unaddressed != UnaddressedChoice::Ignore)
        {
            return Err(validation_err(
                "admission_table",
                "unaddressed cells must be ignore when receive_all is false",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProactiveRule {
    pub rule_id: String,
    pub enabled: bool,
    pub title: String,
    pub description: String,
    pub receiving_scopes: Vec<ReceivingScope>,
    pub writable_scope: Vec<String>,
    pub silent_ack: bool,
}

impl Validate for ProactiveRule {
    fn validate(&self) -> Result<(), ValidationError> {
        require_text("proactive_rules.rule_id", &self.rule_id)?;
        require_text("proactive_rules.title", &self.title)?;
        require_text("proactive_rules.description", &self.description)?;
        if self.receiving_scopes.is_empty() {
            return Err(validation_err(
                "proactive_rules.receiving_scopes",
                "must contain at least one receiving scope",
            ));
        }
        let mut seen_scopes = BTreeSet::new();
        for scope in &self.receiving_scopes {
            if !seen_scopes.insert(*scope) {
                return Err(validation_err(
                    "proactive_rules.receiving_scopes",
                    "must not contain duplicate scopes",
                ));
            }
        }
        validate_safe_scope_list("proactive_rules.writable_scope", &self.writable_scope, true)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdmissionPolicy {
    pub contract_version: String,
    pub accept_mode: AcceptMode,
    pub allowed_actors: Vec<String>,
    pub bot_allowlist: Vec<String>,
    pub automated_reviewers_non_bot: Vec<String>,
    pub admission_table: BTreeMap<ReceivingScope, AdmissionTableRow>,
    pub proactive_rules: Vec<ProactiveRule>,
}

impl Validate for AdmissionPolicy {
    fn validate(&self) -> Result<(), ValidationError> {
        require_version(&self.contract_version)?;

        let allowed = normalize_actor_list("allowed_actors", &self.allowed_actors, true)?;
        for actor in &allowed {
            if is_bot_login(actor) {
                return Err(validation_err(
                    "allowed_actors",
                    "bot logins ending in [bot] must appear in bot_allowlist",
                ));
            }
        }

        let bots = normalize_actor_list("bot_allowlist", &self.bot_allowlist, false)?;
        for bot in &bots {
            if allowed.contains(bot) {
                return Err(validation_err(
                    "bot_allowlist",
                    "actor must not appear in both allowed_actors and bot_allowlist",
                ));
            }
        }

        let reviewers = normalize_actor_list(
            "automated_reviewers_non_bot",
            &self.automated_reviewers_non_bot,
            false,
        )?;
        for reviewer in &reviewers {
            if is_bot_login(reviewer) {
                return Err(validation_err(
                    "automated_reviewers_non_bot",
                    "bot logins ending in [bot] must appear in bot_allowlist",
                ));
            }
            if allowed.contains(reviewer) || bots.contains(reviewer) {
                return Err(validation_err(
                    "automated_reviewers_non_bot",
                    "actor must be disjoint from allowed_actors and bot_allowlist",
                ));
            }
        }

        if self.admission_table.is_empty() {
            return Err(validation_err(
                "admission_table",
                "must contain at least one receiving scope",
            ));
        }
        for (scope, row) in &self.admission_table {
            row.validate_for_scope(*scope)?;
        }

        let mut seen_rules = BTreeSet::new();
        for rule in &self.proactive_rules {
            rule.validate()?;
            if !seen_rules.insert(rule.rule_id.trim().to_string()) {
                return Err(validation_err(
                    "proactive_rules.rule_id",
                    "must not contain duplicate rule ids",
                ));
            }
        }
        Ok(())
    }
}

impl AdmissionPolicy {
    pub fn canonicalized(&self) -> Result<Self, AdmissionError> {
        self.validate()?;
        let allowed_actors = normalize_actor_list("allowed_actors", &self.allowed_actors, true)?
            .into_iter()
            .collect();
        let bot_allowlist = normalize_actor_list("bot_allowlist", &self.bot_allowlist, false)?
            .into_iter()
            .collect();
        let automated_reviewers_non_bot = normalize_actor_list(
            "automated_reviewers_non_bot",
            &self.automated_reviewers_non_bot,
            false,
        )?
        .into_iter()
        .collect();

        let mut proactive_rules: Vec<ProactiveRule> = self
            .proactive_rules
            .iter()
            .map(|r| {
                let scopes: BTreeSet<ReceivingScope> = r.receiving_scopes.iter().copied().collect();
                let writable: BTreeSet<String> = r
                    .writable_scope
                    .iter()
                    .map(|w| canonical_scope_entry(w))
                    .collect();
                ProactiveRule {
                    rule_id: r.rule_id.trim().to_string(),
                    enabled: r.enabled,
                    title: r.title.trim().to_string(),
                    description: r.description.trim().to_string(),
                    receiving_scopes: scopes.into_iter().collect(),
                    writable_scope: writable.into_iter().collect(),
                    silent_ack: r.silent_ack,
                }
            })
            .collect();
        proactive_rules.sort_by(|a, b| a.rule_id.cmp(&b.rule_id));

        Ok(Self {
            contract_version: self.contract_version.trim().to_string(),
            accept_mode: self.accept_mode,
            allowed_actors,
            bot_allowlist,
            automated_reviewers_non_bot,
            admission_table: self.admission_table.clone(),
            proactive_rules,
        })
    }

    /// Computes a deterministic `sha256:<64 hex>` digest over sorted canonical JSON fields.
    pub fn canonical_policy_version(&self) -> Result<String, AdmissionError> {
        #[derive(Serialize)]
        #[serde(untagged)]
        enum CanonValue {
            Bool(bool),
            Str(String),
            Arr(Vec<CanonValue>),
            Obj(BTreeMap<&'static str, CanonValue>),
            DynObj(BTreeMap<String, CanonValue>),
        }

        fn scope_key(scope: ReceivingScope) -> String {
            match scope {
                ReceivingScope::GithubComment => "github_comment",
                ReceivingScope::GithubDefaultBranchCi => "github_default_branch_ci",
                ReceivingScope::GithubIssue => "github_issue",
                ReceivingScope::GithubPr => "github_pr",
                ReceivingScope::OwnerCli => "owner_cli",
            }
            .to_string()
        }

        fn accept_mode_str(mode: AcceptMode) -> &'static str {
            match mode {
                AcceptMode::Off => "off",
                AcceptMode::Observe => "observe",
                AcceptMode::Auto => "auto",
            }
        }

        fn admission_choice_str(choice: AdmissionChoice) -> &'static str {
            match choice {
                AdmissionChoice::Explicit => "explicit",
                AdmissionChoice::Inferred => "inferred",
                AdmissionChoice::Ignore => "ignore",
            }
        }

        fn unaddressed_choice_str(choice: UnaddressedChoice) -> &'static str {
            match choice {
                UnaddressedChoice::Inferred => "inferred",
                UnaddressedChoice::Ignore => "ignore",
            }
        }

        let canon = self.canonicalized()?;
        let mut table_map = BTreeMap::new();
        for (scope, row) in &canon.admission_table {
            let mut row_map = BTreeMap::new();
            row_map.insert(
                "listed_addressed",
                CanonValue::Str(admission_choice_str(row.listed_addressed).into()),
            );
            row_map.insert(
                "listed_unaddressed",
                CanonValue::Str(unaddressed_choice_str(row.listed_unaddressed).into()),
            );
            row_map.insert("receive_all", CanonValue::Bool(row.receive_all));
            row_map.insert(
                "unlisted_addressed",
                CanonValue::Str(admission_choice_str(row.unlisted_addressed).into()),
            );
            row_map.insert(
                "unlisted_unaddressed",
                CanonValue::Str(unaddressed_choice_str(row.unlisted_unaddressed).into()),
            );
            table_map.insert(scope_key(*scope), CanonValue::Obj(row_map));
        }

        let rules_arr: Vec<CanonValue> = canon
            .proactive_rules
            .iter()
            .map(|r| {
                let mut rm = BTreeMap::new();
                rm.insert("description", CanonValue::Str(r.description.clone()));
                rm.insert("enabled", CanonValue::Bool(r.enabled));
                rm.insert(
                    "receiving_scopes",
                    CanonValue::Arr(
                        r.receiving_scopes
                            .iter()
                            .map(|s| CanonValue::Str(scope_key(*s)))
                            .collect(),
                    ),
                );
                rm.insert("rule_id", CanonValue::Str(r.rule_id.clone()));
                rm.insert("silent_ack", CanonValue::Bool(r.silent_ack));
                rm.insert("title", CanonValue::Str(r.title.clone()));
                rm.insert(
                    "writable_scope",
                    CanonValue::Arr(
                        r.writable_scope
                            .iter()
                            .map(|w| CanonValue::Str(w.clone()))
                            .collect(),
                    ),
                );
                CanonValue::Obj(rm)
            })
            .collect();

        let mut root = BTreeMap::new();
        root.insert(
            "accept_mode",
            CanonValue::Str(accept_mode_str(canon.accept_mode).into()),
        );
        root.insert("admission_table", CanonValue::DynObj(table_map));
        root.insert(
            "allowed_actors",
            CanonValue::Arr(
                canon
                    .allowed_actors
                    .into_iter()
                    .map(CanonValue::Str)
                    .collect(),
            ),
        );
        root.insert(
            "automated_reviewers_non_bot",
            CanonValue::Arr(
                canon
                    .automated_reviewers_non_bot
                    .into_iter()
                    .map(CanonValue::Str)
                    .collect(),
            ),
        );
        root.insert(
            "bot_allowlist",
            CanonValue::Arr(
                canon
                    .bot_allowlist
                    .into_iter()
                    .map(CanonValue::Str)
                    .collect(),
            ),
        );
        root.insert("contract_version", CanonValue::Str(canon.contract_version));
        root.insert("proactive_rules", CanonValue::Arr(rules_arr));

        let serialized = serde_json::to_string(&CanonValue::Obj(root))
            .map_err(|e| AdmissionError::Policy(format!("canonical serialization failed: {e}")))?;
        Ok(format!("sha256:{}", sha256_hex(serialized.as_bytes())))
    }

    pub fn is_actor_allowed(&self, actor: &str) -> bool {
        let norm = normalize_actor(actor);
        self.allowed_actors
            .iter()
            .any(|item| normalize_actor(item) == norm)
    }

    pub fn is_bot_allowed(&self, actor: &str) -> bool {
        let norm = normalize_actor(actor);
        self.bot_allowlist
            .iter()
            .any(|item| normalize_actor(item) == norm)
    }

    pub fn is_non_bot_automated_reviewer(&self, actor: &str) -> bool {
        let norm = normalize_actor(actor);
        self.automated_reviewers_non_bot
            .iter()
            .any(|item| normalize_actor(item) == norm)
    }

    pub fn rule(&self, rule_id: &str) -> Option<&ProactiveRule> {
        let target = rule_id.trim();
        self.proactive_rules
            .iter()
            .find(|r| r.rule_id.trim() == target)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorizationMandate {
    pub contract_version: String,
    pub mandate_id: String,
    pub authority_id: String,
    pub origin_thread_id: String,
    pub source_kind: TriggerSourceKind,
    pub receiving_scope: ReceivingScope,
    pub requester_actor: String,
    pub requester_is_bot: bool,
    pub strength: MandateStrength,
    pub policy_version: String,
    pub matched_rules: Vec<String>,
    pub writable_scope: Vec<String>,
    pub status: MandateStatus,
    pub issued_at: String,
}

impl Validate for AuthorizationMandate {
    fn validate(&self) -> Result<(), ValidationError> {
        require_version(&self.contract_version)?;
        require_text("mandate_id", &self.mandate_id)?;
        require_text("authority_id", &self.authority_id)?;
        require_text("origin_thread_id", &self.origin_thread_id)?;
        require_text("requester_actor", &self.requester_actor)?;
        require_policy_version("policy_version", &self.policy_version)?;
        require_timestamp("issued_at", &self.issued_at)?;

        let is_ci_source = self.source_kind == TriggerSourceKind::GithubDefaultBranchCi;
        let is_ci_scope = self.receiving_scope == ReceivingScope::GithubDefaultBranchCi;
        if is_ci_source != is_ci_scope {
            return Err(validation_err(
                "receiving_scope",
                "github_default_branch_ci source_kind and receiving_scope must be paired",
            ));
        }

        let bot_identity = self.requester_is_bot
            || self.source_kind == TriggerSourceKind::Bot
            || is_bot_login(&self.requester_actor);
        if bot_identity && self.strength == MandateStrength::Explicit {
            return Err(validation_err(
                "strength",
                "bot identities cannot hold explicit mandate strength",
            ));
        }
        if is_ci_source && self.strength == MandateStrength::Explicit {
            return Err(validation_err(
                "strength",
                "default-branch CI triggers cannot hold explicit mandate strength",
            ));
        }
        if self.strength == MandateStrength::Inferred && self.matched_rules.is_empty() {
            return Err(validation_err(
                "matched_rules",
                "inferred mandate must record at least one matched proactive rule",
            ));
        }

        let mut seen_rules = BTreeSet::new();
        for rule in &self.matched_rules {
            require_text("matched_rules", rule)?;
            if !seen_rules.insert(rule.trim().to_string()) {
                return Err(validation_err(
                    "matched_rules",
                    "must not contain duplicate rule ids",
                ));
            }
        }

        validate_safe_scope_list("writable_scope", &self.writable_scope, true)
    }
}

impl AuthorizationMandate {
    /// Verifies that this mandate binds to an `ovca_types::Authority` record and never
    /// widens writable paths beyond `Authority.scope`.
    pub fn bind_to_authority(&self, authority: &Authority) -> Result<(), AdmissionError> {
        self.validate()?;
        authority.validate()?;
        if self.authority_id != authority.authority_id {
            return Err(AdmissionError::Mandate(format!(
                "mandate authority_id '{}' does not match authority.authority_id '{}'",
                self.authority_id, authority.authority_id
            )));
        }
        for entry in &self.writable_scope {
            if !is_path_within_scope(entry, &authority.scope) {
                return Err(AdmissionError::Mandate(format!(
                    "mandate writable_scope '{entry}' exceeds authority.scope"
                )));
            }
        }
        Ok(())
    }
}

/// Computes the conservative intersection of `writable_scope` prefixes across matched rules.
pub fn intersect_rule_scopes(rules: &[&ProactiveRule]) -> Vec<String> {
    if rules.is_empty() {
        return Vec::new();
    }
    if rules.len() == 1 {
        let unique: BTreeSet<String> = rules[0]
            .writable_scope
            .iter()
            .map(|s| canonical_scope_entry(s))
            .collect();
        return unique.into_iter().collect();
    }

    let mut common = BTreeSet::new();
    for (idx, rule) in rules.iter().enumerate() {
        let other_scopes: Vec<&[String]> = rules
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != idx)
            .map(|(_, r)| r.writable_scope.as_slice())
            .collect();
        for cand in &rule.writable_scope {
            let clean = canonical_scope_entry(cand);
            if other_scopes
                .iter()
                .all(|other| is_path_within_scope(&clean, other))
            {
                common.insert(clean);
            }
        }
    }
    common.into_iter().collect()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MandateRevalidationReport {
    pub mandate_id: String,
    pub valid_to_resume: bool,
    pub resulting_status: MandateStatus,
    pub policy_changed: bool,
    pub current_policy_version: String,
    pub reason: String,
}

/// Revalidates an `AuthorizationMandate` against the current `AdmissionPolicy` before resume.
pub fn revalidate_mandate(
    mandate: &AuthorizationMandate,
    policy: &AdmissionPolicy,
) -> Result<MandateRevalidationReport, AdmissionError> {
    mandate.validate()?;
    policy.validate()?;
    let current_version = policy.canonical_policy_version()?;
    let policy_changed = mandate.policy_version != current_version;

    match mandate.status {
        MandateStatus::StoppedByPerson => {
            return Ok(MandateRevalidationReport {
                mandate_id: mandate.mandate_id.clone(),
                valid_to_resume: false,
                resulting_status: MandateStatus::StoppedByPerson,
                policy_changed,
                current_policy_version: current_version,
                reason: "mandate was explicitly stopped by a person and cannot resume".into(),
            });
        }
        MandateStatus::Revoked => {
            return Ok(MandateRevalidationReport {
                mandate_id: mandate.mandate_id.clone(),
                valid_to_resume: false,
                resulting_status: MandateStatus::Revoked,
                policy_changed,
                current_policy_version: current_version,
                reason: "mandate is already revoked".into(),
            });
        }
        MandateStatus::Active | MandateStatus::Ended => {}
    }

    match mandate.strength {
        MandateStrength::Explicit => {
            if policy.is_non_bot_automated_reviewer(&mandate.requester_actor)
                || mandate.requester_is_bot
                || mandate.source_kind == TriggerSourceKind::Bot
                || is_bot_login(&mandate.requester_actor)
                || mandate.source_kind == TriggerSourceKind::GithubDefaultBranchCi
                || mandate.receiving_scope == ReceivingScope::GithubDefaultBranchCi
            {
                return Ok(MandateRevalidationReport {
                    mandate_id: mandate.mandate_id.clone(),
                    valid_to_resume: false,
                    resulting_status: MandateStatus::Revoked,
                    policy_changed,
                    current_policy_version: current_version,
                    reason: "explicit mandate disallowed for automated, bot, or CI surface".into(),
                });
            }
            let row = policy.admission_table.get(&mandate.receiving_scope);
            let allowed_explicit = if policy.is_actor_allowed(&mandate.requester_actor) {
                row.is_some_and(|r| r.listed_addressed == AdmissionChoice::Explicit)
            } else {
                row.is_some_and(|r| r.unlisted_addressed == AdmissionChoice::Explicit)
            };
            if !allowed_explicit {
                return Ok(MandateRevalidationReport {
                    mandate_id: mandate.mandate_id.clone(),
                    valid_to_resume: false,
                    resulting_status: MandateStatus::Revoked,
                    policy_changed,
                    current_policy_version: current_version,
                    reason:
                        "requester no longer qualifies for explicit admission under current policy"
                            .into(),
                });
            }
            Ok(MandateRevalidationReport {
                mandate_id: mandate.mandate_id.clone(),
                valid_to_resume: true,
                resulting_status: mandate.status,
                policy_changed,
                current_policy_version: current_version,
                reason: if policy_changed {
                    "explicit mandate remains valid; policy_changed notice attached".into()
                } else {
                    "explicit mandate valid under current policy".into()
                },
            })
        }
        MandateStrength::Inferred => {
            if policy.accept_mode != AcceptMode::Auto {
                return Ok(MandateRevalidationReport {
                    mandate_id: mandate.mandate_id.clone(),
                    valid_to_resume: false,
                    resulting_status: MandateStatus::Revoked,
                    policy_changed,
                    current_policy_version: current_version,
                    reason: "inferred mandate requires accept_mode=auto to resume".into(),
                });
            }

            let mut matched_rule_objs = Vec::with_capacity(mandate.matched_rules.len());
            for rule_id in &mandate.matched_rules {
                match policy.rule(rule_id) {
                    Some(rule) if rule.enabled => {
                        if !rule.receiving_scopes.contains(&mandate.receiving_scope) {
                            return Ok(MandateRevalidationReport {
                                mandate_id: mandate.mandate_id.clone(),
                                valid_to_resume: false,
                                resulting_status: MandateStatus::Revoked,
                                policy_changed,
                                current_policy_version: current_version,
                                reason: format!(
                                    "matched rule '{rule_id}' no longer applies to receiving scope"
                                ),
                            });
                        }
                        matched_rule_objs.push(rule);
                    }
                    Some(_) => {
                        return Ok(MandateRevalidationReport {
                            mandate_id: mandate.mandate_id.clone(),
                            valid_to_resume: false,
                            resulting_status: MandateStatus::Revoked,
                            policy_changed,
                            current_policy_version: current_version,
                            reason: format!("matched rule '{rule_id}' is disabled"),
                        });
                    }
                    None => {
                        return Ok(MandateRevalidationReport {
                            mandate_id: mandate.mandate_id.clone(),
                            valid_to_resume: false,
                            resulting_status: MandateStatus::Revoked,
                            policy_changed,
                            current_policy_version: current_version,
                            reason: format!("matched rule '{rule_id}' was removed from policy"),
                        });
                    }
                }
            }

            let bounded_scope = intersect_rule_scopes(&matched_rule_objs);
            if bounded_scope.is_empty() {
                return Ok(MandateRevalidationReport {
                    mandate_id: mandate.mandate_id.clone(),
                    valid_to_resume: false,
                    resulting_status: MandateStatus::Revoked,
                    policy_changed,
                    current_policy_version: current_version,
                    reason: "matched rules have disjoint writable_scope under current policy"
                        .into(),
                });
            }
            for entry in &mandate.writable_scope {
                if !is_path_within_scope(entry, &bounded_scope) {
                    return Ok(MandateRevalidationReport {
                        mandate_id: mandate.mandate_id.clone(),
                        valid_to_resume: false,
                        resulting_status: MandateStatus::Revoked,
                        policy_changed,
                        current_policy_version: current_version,
                        reason: format!(
                            "mandate writable_scope '{entry}' exceeds narrowed rule scope"
                        ),
                    });
                }
            }

            if mandate.source_kind == TriggerSourceKind::GithubDefaultBranchCi
                || mandate.receiving_scope == ReceivingScope::GithubDefaultBranchCi
            {
                let ci_enabled = policy
                    .admission_table
                    .get(&ReceivingScope::GithubDefaultBranchCi)
                    .is_some_and(|r| r.unlisted_unaddressed == UnaddressedChoice::Inferred);
                if !ci_enabled {
                    return Ok(MandateRevalidationReport {
                        mandate_id: mandate.mandate_id.clone(),
                        valid_to_resume: false,
                        resulting_status: MandateStatus::Revoked,
                        policy_changed,
                        current_policy_version: current_version,
                        reason: "github_default_branch_ci cell set to ignore".into(),
                    });
                }
            } else if mandate.requester_is_bot
                || mandate.source_kind == TriggerSourceKind::Bot
                || is_bot_login(&mandate.requester_actor)
            {
                if !policy.is_bot_allowed(&mandate.requester_actor)
                    || !policy
                        .admission_table
                        .contains_key(&mandate.receiving_scope)
                {
                    return Ok(MandateRevalidationReport {
                        mandate_id: mandate.mandate_id.clone(),
                        valid_to_resume: false,
                        resulting_status: MandateStatus::Revoked,
                        policy_changed,
                        current_policy_version: current_version,
                        reason: "bot removed from bot_allowlist or receiving scope missing".into(),
                    });
                }
            } else if policy.is_non_bot_automated_reviewer(&mandate.requester_actor) {
                if !policy
                    .admission_table
                    .contains_key(&mandate.receiving_scope)
                {
                    return Ok(MandateRevalidationReport {
                        mandate_id: mandate.mandate_id.clone(),
                        valid_to_resume: false,
                        resulting_status: MandateStatus::Revoked,
                        policy_changed,
                        current_policy_version: current_version,
                        reason: "receiving scope missing for automated reviewer".into(),
                    });
                }
            } else {
                let row = policy.admission_table.get(&mandate.receiving_scope);
                let still_admitted = row.is_some_and(|r| {
                    if policy.is_actor_allowed(&mandate.requester_actor) {
                        r.listed_addressed != AdmissionChoice::Ignore
                            || (r.receive_all
                                && r.listed_unaddressed == UnaddressedChoice::Inferred)
                    } else {
                        r.unlisted_addressed != AdmissionChoice::Ignore
                            || (r.receive_all
                                && r.unlisted_unaddressed == UnaddressedChoice::Inferred)
                    }
                });
                if !still_admitted {
                    return Ok(MandateRevalidationReport {
                        mandate_id: mandate.mandate_id.clone(),
                        valid_to_resume: false,
                        resulting_status: MandateStatus::Revoked,
                        policy_changed,
                        current_policy_version: current_version,
                        reason: "admission table weakened for requester".into(),
                    });
                }
            }

            Ok(MandateRevalidationReport {
                mandate_id: mandate.mandate_id.clone(),
                valid_to_resume: true,
                resulting_status: mandate.status,
                policy_changed,
                current_policy_version: current_version,
                reason: if policy_changed {
                    "inferred mandate remains valid; policy_changed notice attached".into()
                } else {
                    "inferred mandate valid under current policy".into()
                },
            })
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FollowUpEvent {
    pub trigger_id: String,
    pub thread_id: String,
    pub subscription_mode: SubscriptionMode,
    pub actor: String,
    pub actor_is_bot: bool,
    pub is_ci: bool,
    pub addressed: AddressedKind,
    pub requested_scope_paths: Vec<String>,
    pub requests_stop: bool,
    pub requests_scope_expansion: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FollowUpAction {
    ProceedInScope,
    ContextOnlyDeliver,
    HandbackRequired,
    StopRequested,
    RevokedCannotResume,
    IgnoreStoppedOrRevoked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HandbackRecord {
    pub mandate_id: String,
    pub trigger_id: String,
    pub preserve_subscription: bool,
    pub out_of_scope_paths: Vec<String>,
    pub route_to: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FollowUpDecision {
    pub action: FollowUpAction,
    pub grant_authority: bool,
    pub allow_resume: bool,
    pub policy_changed_notice: bool,
    pub out_of_scope_paths: Vec<String>,
    pub handback: Option<HandbackRecord>,
    pub reason: String,
}

/// Evaluates a follow-up event under `Context != Authority`, keeping mandates immutable.
pub fn evaluate_followup(
    mandate: &AuthorizationMandate,
    followup: &FollowUpEvent,
    policy: &AdmissionPolicy,
) -> Result<FollowUpDecision, AdmissionError> {
    mandate.validate()?;
    policy.validate()?;
    require_text("followup.trigger_id", &followup.trigger_id)?;
    require_text("followup.thread_id", &followup.thread_id)?;
    require_text("followup.actor", &followup.actor)?;

    let reval = revalidate_mandate(mandate, policy)?;

    if matches!(
        mandate.status,
        MandateStatus::StoppedByPerson | MandateStatus::Revoked
    ) {
        return Ok(FollowUpDecision {
            action: FollowUpAction::IgnoreStoppedOrRevoked,
            grant_authority: false,
            allow_resume: false,
            policy_changed_notice: reval.policy_changed,
            out_of_scope_paths: Vec::new(),
            handback: None,
            reason: reval.reason,
        });
    }

    if followup.subscription_mode == SubscriptionMode::ContextOnly
        || followup.thread_id != mandate.origin_thread_id
    {
        return Ok(FollowUpDecision {
            action: FollowUpAction::ContextOnlyDeliver,
            grant_authority: false,
            allow_resume: false,
            policy_changed_notice: reval.policy_changed,
            out_of_scope_paths: Vec::new(),
            handback: None,
            reason: "context-only subscription or secondary thread grants no authority".into(),
        });
    }

    let is_automated = followup.actor_is_bot
        || followup.is_ci
        || is_bot_login(&followup.actor)
        || policy.is_non_bot_automated_reviewer(&followup.actor);
    if is_automated {
        return Ok(FollowUpDecision {
            action: FollowUpAction::ContextOnlyDeliver,
            grant_authority: false,
            allow_resume: false,
            policy_changed_notice: reval.policy_changed,
            out_of_scope_paths: Vec::new(),
            handback: None,
            reason: "automated, bot, or CI follow-up is delivered as context only".into(),
        });
    }

    let is_listed = policy.is_actor_allowed(&followup.actor);
    let row = policy.admission_table.get(&mandate.receiving_scope);
    let is_addressed = matches!(
        followup.addressed,
        AddressedKind::Mention | AddressedKind::Direct
    );

    let is_explicit_actor = if is_listed {
        mandate.receiving_scope == ReceivingScope::GithubDefaultBranchCi
            || row.is_some_and(|r| r.listed_addressed == AdmissionChoice::Explicit)
    } else {
        is_addressed && row.is_some_and(|r| r.unlisted_addressed == AdmissionChoice::Explicit)
    };

    if !is_explicit_actor {
        if policy.accept_mode != AcceptMode::Auto {
            return Ok(FollowUpDecision {
                action: FollowUpAction::ContextOnlyDeliver,
                grant_authority: false,
                allow_resume: false,
                policy_changed_notice: reval.policy_changed,
                out_of_scope_paths: Vec::new(),
                handback: None,
                reason: "non-explicit follow-up yields context only unless accept_mode is auto"
                    .into(),
            });
        }
        if mandate.receiving_scope == ReceivingScope::GithubDefaultBranchCi {
            return Ok(FollowUpDecision {
                action: FollowUpAction::ContextOnlyDeliver,
                grant_authority: false,
                allow_resume: false,
                policy_changed_notice: reval.policy_changed,
                out_of_scope_paths: Vec::new(),
                handback: None,
                reason: "unlisted actor cannot act on default-branch CI mandate".into(),
            });
        }
        let inferred_admitted = row.is_some_and(|r| {
            if is_listed {
                if is_addressed {
                    r.listed_addressed != AdmissionChoice::Ignore
                } else {
                    r.listed_addressed != AdmissionChoice::Ignore
                        || (r.receive_all && r.listed_unaddressed == UnaddressedChoice::Inferred)
                }
            } else if is_addressed {
                r.unlisted_addressed != AdmissionChoice::Ignore
            } else {
                r.receive_all && r.unlisted_unaddressed == UnaddressedChoice::Inferred
            }
        });
        if !inferred_admitted {
            return Ok(FollowUpDecision {
                action: FollowUpAction::ContextOnlyDeliver,
                grant_authority: false,
                allow_resume: false,
                policy_changed_notice: reval.policy_changed,
                out_of_scope_paths: Vec::new(),
                handback: None,
                reason: "actor is not admitted by admission_table for this receiving scope".into(),
            });
        }
    }

    if followup.requests_stop {
        if !is_explicit_actor {
            return Ok(FollowUpDecision {
                action: FollowUpAction::ContextOnlyDeliver,
                grant_authority: false,
                allow_resume: false,
                policy_changed_notice: reval.policy_changed,
                out_of_scope_paths: Vec::new(),
                handback: None,
                reason: "non-explicit actor cannot stop a mandate".into(),
            });
        }
        return Ok(FollowUpDecision {
            action: FollowUpAction::StopRequested,
            grant_authority: false,
            allow_resume: false,
            policy_changed_notice: reval.policy_changed,
            out_of_scope_paths: Vec::new(),
            handback: None,
            reason: "explicit human stop request accepted".into(),
        });
    }

    if !reval.valid_to_resume {
        return Ok(FollowUpDecision {
            action: FollowUpAction::RevokedCannotResume,
            grant_authority: false,
            allow_resume: false,
            policy_changed_notice: reval.policy_changed,
            out_of_scope_paths: Vec::new(),
            handback: None,
            reason: reval.reason,
        });
    }

    let out_of_scope: Vec<String> = followup
        .requested_scope_paths
        .iter()
        .filter(|path| !is_path_within_scope(path, &mandate.writable_scope))
        .cloned()
        .collect();

    if followup.requests_scope_expansion || !out_of_scope.is_empty() {
        let reason = if !out_of_scope.is_empty() {
            format!(
                "requested paths exceed mandate writable_scope: {}",
                out_of_scope.join(", ")
            )
        } else {
            "follow-up requested scope expansion beyond immutable mandate".into()
        };
        return Ok(FollowUpDecision {
            action: FollowUpAction::HandbackRequired,
            grant_authority: false,
            allow_resume: false,
            policy_changed_notice: reval.policy_changed,
            out_of_scope_paths: out_of_scope.clone(),
            handback: Some(HandbackRecord {
                mandate_id: mandate.mandate_id.clone(),
                trigger_id: followup.trigger_id.clone(),
                preserve_subscription: true,
                out_of_scope_paths: out_of_scope,
                route_to: "fresh_eval".into(),
                reason: reason.clone(),
            }),
            reason,
        });
    }

    Ok(FollowUpDecision {
        action: FollowUpAction::ProceedInScope,
        grant_authority: true,
        allow_resume: mandate.status == MandateStatus::Ended,
        policy_changed_notice: reval.policy_changed,
        out_of_scope_paths: Vec::new(),
        handback: None,
        reason: if mandate.status == MandateStatus::Ended {
            "human follow-up passed revalidation and resumed within mandate writable_scope".into()
        } else {
            "human follow-up fits active mandate writable_scope".into()
        },
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdmissionDecisionKind {
    Accept,
    Stop,
    Ignore,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProactiveTrigger {
    pub trigger_id: String,
    pub thread_id: String,
    pub source_kind: TriggerSourceKind,
    pub receiving_scope: ReceivingScope,
    pub actor: String,
    pub actor_is_bot: bool,
    pub addressed: AddressedKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleMatchCandidate {
    pub rule_id: String,
    pub matched: bool,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum AssessmentInput {
    Evaluated { candidates: Vec<RuleMatchCandidate> },
    Unavailable { error_kind: String, reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProactiveDecisionMeta {
    pub decision_id: String,
    pub mandate_id: Option<String>,
    pub authority_id: Option<String>,
    pub recorded_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdmissionDecisionRecord {
    pub contract_version: String,
    pub decision_id: String,
    pub trigger_id: String,
    pub thread_id: String,
    pub decision: AdmissionDecisionKind,
    pub accept_mode: AcceptMode,
    pub policy_version: String,
    pub would_accept: bool,
    pub matched_rules: Vec<String>,
    pub proposed_writable_scope: Vec<String>,
    pub error_kind: Option<String>,
    pub reason: String,
    pub recorded_at: String,
}

impl Validate for AdmissionDecisionRecord {
    fn validate(&self) -> Result<(), ValidationError> {
        require_version(&self.contract_version)?;
        require_text("decision_id", &self.decision_id)?;
        require_text("trigger_id", &self.trigger_id)?;
        require_text("thread_id", &self.thread_id)?;
        require_policy_version("policy_version", &self.policy_version)?;
        require_text("reason", &self.reason)?;
        require_timestamp("recorded_at", &self.recorded_at)?;

        let mut seen_rules = BTreeSet::new();
        for rule in &self.matched_rules {
            require_text("matched_rules", rule)?;
            if !seen_rules.insert(rule.trim().to_string()) {
                return Err(validation_err(
                    "matched_rules",
                    "must not contain duplicate rule ids",
                ));
            }
        }
        validate_safe_scope_list(
            "proposed_writable_scope",
            &self.proposed_writable_scope,
            false,
        )?;

        match self.decision {
            AdmissionDecisionKind::Unavailable => {
                if self.would_accept {
                    return Err(validation_err(
                        "would_accept",
                        "must be false when decision is unavailable",
                    ));
                }
                if !self.matched_rules.is_empty() || !self.proposed_writable_scope.is_empty() {
                    return Err(validation_err(
                        "matched_rules",
                        "unavailable decision must not claim matched rules or writable scope",
                    ));
                }
                match &self.error_kind {
                    Some(kind) => require_text("error_kind", kind)?,
                    None => {
                        return Err(validation_err(
                            "error_kind",
                            "must be present when decision is unavailable",
                        ))
                    }
                }
            }
            AdmissionDecisionKind::Accept => {
                if self.error_kind.is_some() {
                    return Err(validation_err(
                        "error_kind",
                        "must be null when decision is not unavailable",
                    ));
                }
                if !self.would_accept {
                    return Err(validation_err(
                        "would_accept",
                        "must be true when decision is accept",
                    ));
                }
                if self.accept_mode != AcceptMode::Auto {
                    return Err(validation_err(
                        "accept_mode",
                        "proactive accept requires accept_mode=auto",
                    ));
                }
                if self.matched_rules.is_empty() || self.proposed_writable_scope.is_empty() {
                    return Err(validation_err(
                        "proposed_writable_scope",
                        "accepted proactive decision requires matched_rules and proposed_writable_scope",
                    ));
                }
            }
            AdmissionDecisionKind::Ignore | AdmissionDecisionKind::Stop => {
                if self.error_kind.is_some() {
                    return Err(validation_err(
                        "error_kind",
                        "must be null when decision is not unavailable",
                    ));
                }
                if self.accept_mode == AcceptMode::Off && self.would_accept {
                    return Err(validation_err(
                        "would_accept",
                        "must be false when accept_mode is off",
                    ));
                }
                if self.would_accept
                    && (self.matched_rules.is_empty() || self.proposed_writable_scope.is_empty())
                {
                    return Err(validation_err(
                        "would_accept",
                        "would_accept=true requires non-empty matched_rules and proposed_writable_scope",
                    ));
                }
            }
        }
        Ok(())
    }
}

fn is_proactive_surface_admitted(policy: &AdmissionPolicy, trigger: &ProactiveTrigger) -> bool {
    let is_ci_source = trigger.source_kind == TriggerSourceKind::GithubDefaultBranchCi;
    let is_ci_scope = trigger.receiving_scope == ReceivingScope::GithubDefaultBranchCi;
    if is_ci_source != is_ci_scope {
        return false;
    }
    if is_ci_scope {
        return policy
            .admission_table
            .get(&ReceivingScope::GithubDefaultBranchCi)
            .is_some_and(|r| r.unlisted_unaddressed == UnaddressedChoice::Inferred);
    }

    if trigger.actor_is_bot
        || trigger.source_kind == TriggerSourceKind::Bot
        || is_bot_login(&trigger.actor)
    {
        return policy.is_bot_allowed(&trigger.actor)
            && policy
                .admission_table
                .contains_key(&trigger.receiving_scope);
    }

    if policy.is_non_bot_automated_reviewer(&trigger.actor) {
        return policy
            .admission_table
            .contains_key(&trigger.receiving_scope);
    }

    let Some(row) = policy.admission_table.get(&trigger.receiving_scope) else {
        return false;
    };
    let addressed = matches!(
        trigger.addressed,
        AddressedKind::Mention | AddressedKind::Direct
    );
    if policy.is_actor_allowed(&trigger.actor) {
        if addressed {
            row.listed_addressed != AdmissionChoice::Ignore
        } else {
            row.receive_all && row.listed_unaddressed == UnaddressedChoice::Inferred
        }
    } else if addressed {
        row.unlisted_addressed != AdmissionChoice::Ignore
    } else {
        row.receive_all && row.unlisted_unaddressed == UnaddressedChoice::Inferred
    }
}

/// Evaluates a proactive trigger under `off | observe | auto` without flattening `unavailable`
/// into `ignore`. In `observe` mode, qualifying triggers record `would_accept = true` while
/// returning `None` for the mandate.
pub fn evaluate_proactive_admission(
    policy: &AdmissionPolicy,
    trigger: &ProactiveTrigger,
    assessment: &AssessmentInput,
    meta: &ProactiveDecisionMeta,
) -> Result<(AdmissionDecisionRecord, Option<AuthorizationMandate>), AdmissionError> {
    policy.validate()?;
    require_text("trigger.trigger_id", &trigger.trigger_id)?;
    require_text("trigger.thread_id", &trigger.thread_id)?;
    require_text("trigger.actor", &trigger.actor)?;
    require_text("meta.decision_id", &meta.decision_id)?;
    require_timestamp("meta.recorded_at", &meta.recorded_at)?;

    let policy_version = policy.canonical_policy_version()?;

    if policy.accept_mode == AcceptMode::Off {
        let record = AdmissionDecisionRecord {
            contract_version: CONTRACT_VERSION.into(),
            decision_id: meta.decision_id.clone(),
            trigger_id: trigger.trigger_id.clone(),
            thread_id: trigger.thread_id.clone(),
            decision: AdmissionDecisionKind::Ignore,
            accept_mode: policy.accept_mode,
            policy_version,
            would_accept: false,
            matched_rules: Vec::new(),
            proposed_writable_scope: Vec::new(),
            error_kind: None,
            reason: "proactive admission disabled while accept_mode is off".into(),
            recorded_at: meta.recorded_at.clone(),
        };
        record.validate()?;
        return Ok((record, None));
    }

    let candidates = match assessment {
        AssessmentInput::Unavailable { error_kind, reason } => {
            require_text("assessment.error_kind", error_kind)?;
            require_text("assessment.reason", reason)?;
            let record = AdmissionDecisionRecord {
                contract_version: CONTRACT_VERSION.into(),
                decision_id: meta.decision_id.clone(),
                trigger_id: trigger.trigger_id.clone(),
                thread_id: trigger.thread_id.clone(),
                decision: AdmissionDecisionKind::Unavailable,
                accept_mode: policy.accept_mode,
                policy_version,
                would_accept: false,
                matched_rules: Vec::new(),
                proposed_writable_scope: Vec::new(),
                error_kind: Some(error_kind.trim().to_string()),
                reason: reason.trim().to_string(),
                recorded_at: meta.recorded_at.clone(),
            };
            record.validate()?;
            return Ok((record, None));
        }
        AssessmentInput::Evaluated { candidates } => candidates,
    };

    if !is_proactive_surface_admitted(policy, trigger) {
        let record = AdmissionDecisionRecord {
            contract_version: CONTRACT_VERSION.into(),
            decision_id: meta.decision_id.clone(),
            trigger_id: trigger.trigger_id.clone(),
            thread_id: trigger.thread_id.clone(),
            decision: AdmissionDecisionKind::Ignore,
            accept_mode: policy.accept_mode,
            policy_version,
            would_accept: false,
            matched_rules: Vec::new(),
            proposed_writable_scope: Vec::new(),
            error_kind: None,
            reason: "trigger actor or receiving scope is not admitted by policy".into(),
            recorded_at: meta.recorded_at.clone(),
        };
        record.validate()?;
        return Ok((record, None));
    }

    let mut matched_ids = BTreeSet::new();
    let mut matched_rules = Vec::new();
    for cand in candidates {
        require_text("candidates.rule_id", &cand.rule_id)?;
        require_text("candidates.reason", &cand.reason)?;
        if !cand.matched {
            continue;
        }
        if let Some(rule) = policy.rule(&cand.rule_id) {
            if rule.enabled
                && rule.receiving_scopes.contains(&trigger.receiving_scope)
                && matched_ids.insert(rule.rule_id.trim().to_string())
            {
                matched_rules.push(rule);
            }
        }
    }
    matched_rules.sort_by(|a, b| a.rule_id.cmp(&b.rule_id));

    if matched_rules.is_empty() {
        let record = AdmissionDecisionRecord {
            contract_version: CONTRACT_VERSION.into(),
            decision_id: meta.decision_id.clone(),
            trigger_id: trigger.trigger_id.clone(),
            thread_id: trigger.thread_id.clone(),
            decision: AdmissionDecisionKind::Ignore,
            accept_mode: policy.accept_mode,
            policy_version,
            would_accept: false,
            matched_rules: Vec::new(),
            proposed_writable_scope: Vec::new(),
            error_kind: None,
            reason: "no enabled proactive rule matched this trigger".into(),
            recorded_at: meta.recorded_at.clone(),
        };
        record.validate()?;
        return Ok((record, None));
    }

    let proposed_writable_scope = intersect_rule_scopes(&matched_rules);
    if proposed_writable_scope.is_empty() {
        let record = AdmissionDecisionRecord {
            contract_version: CONTRACT_VERSION.into(),
            decision_id: meta.decision_id.clone(),
            trigger_id: trigger.trigger_id.clone(),
            thread_id: trigger.thread_id.clone(),
            decision: AdmissionDecisionKind::Ignore,
            accept_mode: policy.accept_mode,
            policy_version,
            would_accept: false,
            matched_rules: Vec::new(),
            proposed_writable_scope: Vec::new(),
            error_kind: None,
            reason: "matched proactive rules have disjoint writable_scope".into(),
            recorded_at: meta.recorded_at.clone(),
        };
        record.validate()?;
        return Ok((record, None));
    }

    let matched_rule_ids: Vec<String> = matched_ids.into_iter().collect();

    match policy.accept_mode {
        AcceptMode::Observe => {
            let record = AdmissionDecisionRecord {
                contract_version: CONTRACT_VERSION.into(),
                decision_id: meta.decision_id.clone(),
                trigger_id: trigger.trigger_id.clone(),
                thread_id: trigger.thread_id.clone(),
                decision: AdmissionDecisionKind::Ignore,
                accept_mode: AcceptMode::Observe,
                policy_version,
                would_accept: true,
                matched_rules: matched_rule_ids,
                proposed_writable_scope,
                error_kind: None,
                reason: "observe mode recorded would_accept without issuing a mandate".into(),
                recorded_at: meta.recorded_at.clone(),
            };
            record.validate()?;
            Ok((record, None))
        }
        AcceptMode::Auto => {
            let mandate_id = meta.mandate_id.as_deref().ok_or_else(|| {
                AdmissionError::Mandate(
                    "meta.mandate_id is required when accept_mode=auto accepts a trigger".into(),
                )
            })?;
            let authority_id = meta.authority_id.as_deref().ok_or_else(|| {
                AdmissionError::Mandate(
                    "meta.authority_id is required when accept_mode=auto accepts a trigger".into(),
                )
            })?;
            let mandate = AuthorizationMandate {
                contract_version: CONTRACT_VERSION.into(),
                mandate_id: mandate_id.to_string(),
                authority_id: authority_id.to_string(),
                origin_thread_id: trigger.thread_id.clone(),
                source_kind: trigger.source_kind,
                receiving_scope: trigger.receiving_scope,
                requester_actor: trigger.actor.clone(),
                requester_is_bot: trigger.actor_is_bot || is_bot_login(&trigger.actor),
                strength: MandateStrength::Inferred,
                policy_version: policy_version.clone(),
                matched_rules: matched_rule_ids.clone(),
                writable_scope: proposed_writable_scope.clone(),
                status: MandateStatus::Active,
                issued_at: meta.recorded_at.clone(),
            };
            mandate.validate()?;

            let record = AdmissionDecisionRecord {
                contract_version: CONTRACT_VERSION.into(),
                decision_id: meta.decision_id.clone(),
                trigger_id: trigger.trigger_id.clone(),
                thread_id: trigger.thread_id.clone(),
                decision: AdmissionDecisionKind::Accept,
                accept_mode: AcceptMode::Auto,
                policy_version,
                would_accept: true,
                matched_rules: matched_rule_ids,
                proposed_writable_scope,
                error_kind: None,
                reason: "auto mode accepted trigger and issued bounded inferred mandate".into(),
                recorded_at: meta.recorded_at.clone(),
            };
            record.validate()?;
            Ok((record, Some(mandate)))
        }
        AcceptMode::Off => unreachable!("handled above"),
    }
}
