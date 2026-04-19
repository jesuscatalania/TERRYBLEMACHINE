//! Token / credit budget manager.
//!
//! Tracks cumulative spend per session and per UTC day, with optional hard
//! limits. Answers one question per request: "should this call be blocked?"
//! Also offers a CSV export of every recorded [`UsageEntry`].
//!
//! Cost estimates per model come from `docs/LLM-STRATEGIE.md`. Callers should
//! prefer response-reported costs when available and fall back to
//! [`cost_cents_for`] only when no cost is known.

#[cfg(test)]
use std::time::Duration;

use std::sync::Arc;

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use super::models::{Model, Provider, TaskKind};

// ─── Clock ────────────────────────────────────────────────────────────────

/// Abstraction over "today", so tests can pin the current UTC date and
/// exercise day-rollover logic without depending on wall-clock time.
pub trait Clock: Send + Sync {
    /// The UTC date the manager should treat as "today".
    fn today(&self) -> NaiveDate;
    /// The current instant. Used for session/record timestamps.
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// Production clock backed by `chrono::Utc::now()`.
pub struct SystemClock;

impl Clock for SystemClock {
    fn today(&self) -> NaiveDate {
        Utc::now().date_naive()
    }
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// Fraction of a limit at which [`BudgetState::Warn`] kicks in.
pub const WARN_THRESHOLD: f64 = 0.80;

/// Default daily ceiling (≈ docs/LLM-STRATEGIE.md daily target ~$11.80).
pub const DEFAULT_DAILY_LIMIT_CENTS: u64 = 5000; // $50 to leave headroom

// ─── Types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BudgetState {
    Ok,
    Warn,
    Block,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BudgetLimits {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub daily_cents: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_cents: Option<u64>,
}

impl Default for BudgetLimits {
    fn default() -> Self {
        Self {
            daily_cents: Some(DEFAULT_DAILY_LIMIT_CENTS),
            session_cents: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BudgetStatus {
    pub state: BudgetState,
    pub used_today_cents: u64,
    pub used_session_cents: u64,
    pub limits: BudgetLimits,
    /// Start of the UTC day the `used_today_cents` counter is tracking.
    pub day_started_at: DateTime<Utc>,
    pub session_started_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageEntry {
    pub timestamp: DateTime<Utc>,
    pub provider: Provider,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<Model>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task: Option<TaskKind>,
    pub cost_cents: u64,
}

// ─── Cost table ───────────────────────────────────────────────────────────

/// Indicative per-call cost in cents. Abo-covered models return 0 — they
/// consume credits, not dollars, and the manager treats them as free against
/// the USD budget ceiling.
///
/// Source: `docs/LLM-STRATEGIE.md` §"Modell-Zuordnung nach Aufgabe". Numbers
/// are rounded up slightly so we err on the side of over-counting.
pub fn cost_cents_for(model: Model) -> u64 {
    // Rough per-unit estimates — replace with live-provider costs when available.
    match model {
        // Claude Max — flat-fee abo.
        Model::ClaudeOpus | Model::ClaudeSonnet | Model::ClaudeHaiku => 0,
        // Video abos — credits only, but we bookkeep rough USD equivalents so
        // the daily budget ceiling protects against runaway requests before a
        // user's aboset runs dry.
        Model::Kling20 => 15,    // ≈ $0.15 per 5s clip (abo credit equivalent)
        Model::RunwayGen3 => 50, // ≈ $0.50 per 5s clip
        Model::HiggsfieldMulti => 30, // ≈ $0.30 per clip
        Model::ShotstackMontage => 10, // ≈ $0.10 per render
        // Image abos / pay-per-use.
        Model::IdeogramV3 => 0,
        Model::MeshyText3D | Model::MeshyImage3D => 0,
        Model::FalFluxPro => 3,
        Model::FalSdxl => 1, // ≈ $0.003, rounded up to 1¢
        Model::FalRealEsrgan => 1,
        Model::FalFluxFill => 3,
        // Kling via fal.ai aggregator — pay-per-use, billed by clip length.
        // Rough per-5s clip list prices (2026-04): V1.5 standard ≈ $0.15
        // → 15¢; V2 master ≈ $0.30 → 30¢.
        Model::FalKlingV15 => 15,
        Model::FalKlingV2Master => 30,
        Model::ReplicateFluxDev => 3,
        // Depth-Anything v2 large on Replicate: ≈ $0.008 per prediction,
        // rounded up to 1¢.
        Model::ReplicateDepthAnythingV2 => 1,
        // TripoSR on Replicate: quick image-to-3D preview. Roughly $0.01–0.02
        // per run depending on host; rounded up to 2¢ — cheaper than Meshy.
        Model::ReplicateTripoSR => 2,
    }
}

// ─── Manager ──────────────────────────────────────────────────────────────

pub struct BudgetManager {
    inner: Mutex<Inner>,
    clock: Arc<dyn Clock>,
}

struct Inner {
    limits: BudgetLimits,
    used_today_cents: u64,
    used_session_cents: u64,
    day_started_at: NaiveDate,
    session_started_at: DateTime<Utc>,
    entries: Vec<UsageEntry>,
}

impl BudgetManager {
    pub fn new(limits: BudgetLimits) -> Self {
        Self::with_clock(limits, Arc::new(SystemClock))
    }

    /// Like [`BudgetManager::new`], but with an injected clock. Production
    /// code uses [`SystemClock`]; tests pin "today" with a manual clock to
    /// exercise day-rollover logic deterministically.
    pub fn with_clock(limits: BudgetLimits, clock: Arc<dyn Clock>) -> Self {
        let today = clock.today();
        let now = clock.now();
        Self {
            inner: Mutex::new(Inner {
                limits,
                used_today_cents: 0,
                used_session_cents: 0,
                day_started_at: today,
                session_started_at: now,
                entries: Vec::new(),
            }),
            clock,
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(BudgetLimits::default())
    }

    /// Record a completed call. `cost_cents == 0` is still recorded so the
    /// CSV export captures every request, even free ones.
    pub async fn record(&self, entry: UsageEntry) {
        let today = self.clock.today();
        let mut inner = self.inner.lock().await;
        inner.roll_day_if_needed(today);
        inner.used_today_cents = inner.used_today_cents.saturating_add(entry.cost_cents);
        inner.used_session_cents = inner.used_session_cents.saturating_add(entry.cost_cents);
        inner.entries.push(entry);
    }

    pub async fn status(&self) -> BudgetStatus {
        let today = self.clock.today();
        let mut inner = self.inner.lock().await;
        inner.roll_day_if_needed(today);
        let state = compute_state(
            inner.used_today_cents,
            inner.used_session_cents,
            &inner.limits,
        );
        BudgetStatus {
            state,
            used_today_cents: inner.used_today_cents,
            used_session_cents: inner.used_session_cents,
            limits: inner.limits,
            day_started_at: start_of_utc_day_for(inner.day_started_at),
            session_started_at: inner.session_started_at,
        }
    }

    pub async fn set_limits(&self, limits: BudgetLimits) {
        self.inner.lock().await.limits = limits;
    }

    /// Returns true if a call with the indicative `projected_cents` cost
    /// would push daily or session spend over 100 %.
    pub async fn would_block(&self, projected_cents: u64) -> bool {
        let today = self.clock.today();
        let mut inner = self.inner.lock().await;
        inner.roll_day_if_needed(today);
        let projected_today = inner.used_today_cents.saturating_add(projected_cents);
        let projected_session = inner.used_session_cents.saturating_add(projected_cents);
        match compute_state(projected_today, projected_session, &inner.limits) {
            BudgetState::Block => true,
            BudgetState::Warn | BudgetState::Ok => false,
        }
    }

    pub async fn entries(&self) -> Vec<UsageEntry> {
        self.inner.lock().await.entries.clone()
    }

    pub async fn export_csv(&self) -> String {
        let entries = self.inner.lock().await.entries.clone();
        let mut out = String::from("timestamp,provider,model,task,cost_cents\n");
        for e in entries {
            let model = e
                .model
                .map(|m| serde_json::to_string(&m).unwrap_or_default())
                .unwrap_or_else(|| "\"\"".to_string());
            let task = e
                .task
                .map(|t| serde_json::to_string(&t).unwrap_or_default())
                .unwrap_or_else(|| "\"\"".to_string());
            let provider = serde_json::to_string(&e.provider).unwrap_or_default();
            out.push_str(&format!(
                "{},{},{},{},{}\n",
                e.timestamp.to_rfc3339(),
                provider.trim_matches('"'),
                model.trim_matches('"'),
                task.trim_matches('"'),
                e.cost_cents,
            ));
        }
        out
    }

    /// Reset the session counter but keep daily counters intact.
    pub async fn start_new_session(&self) {
        let now = self.clock.now();
        let mut inner = self.inner.lock().await;
        inner.used_session_cents = 0;
        inner.session_started_at = now;
    }

    /// Admin / testing reset.
    pub async fn clear(&self) {
        let today = self.clock.today();
        let now = self.clock.now();
        let mut inner = self.inner.lock().await;
        inner.used_today_cents = 0;
        inner.used_session_cents = 0;
        inner.entries.clear();
        inner.day_started_at = today;
        inner.session_started_at = now;
    }
}

impl Default for BudgetManager {
    fn default() -> Self {
        Self::with_defaults()
    }
}

impl Inner {
    fn roll_day_if_needed(&mut self, today: NaiveDate) {
        if today > self.day_started_at {
            self.used_today_cents = 0;
            self.day_started_at = today;
        }
    }
}

fn compute_state(used_today: u64, used_session: u64, limits: &BudgetLimits) -> BudgetState {
    let mut state = BudgetState::Ok;
    for (used, limit) in [
        (used_today, limits.daily_cents),
        (used_session, limits.session_cents),
    ] {
        let Some(limit) = limit else { continue };
        if limit == 0 {
            continue;
        }
        let ratio = used as f64 / limit as f64;
        if ratio >= 1.0 {
            return BudgetState::Block;
        } else if ratio >= WARN_THRESHOLD {
            state = BudgetState::Warn;
        }
    }
    state
}

/// Convert a [`NaiveDate`] to the UTC instant at its 00:00. Used for the
/// `BudgetStatus::day_started_at` field, which still reports an absolute
/// timestamp for downstream consumers.
fn start_of_utc_day_for(date: NaiveDate) -> DateTime<Utc> {
    date.and_hms_opt(0, 0, 0)
        .map(|ndt| ndt.and_utc())
        .unwrap_or_else(Utc::now)
}

/// Convert an elapsed [`Duration`] into a friendlier `Hh Mm` string.
#[cfg(test)]
fn human_duration(d: Duration) -> String {
    let secs = d.as_secs();
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    if h > 0 {
        format!("{h}h {m}m")
    } else {
        format!("{m}m")
    }
}

#[cfg(test)]
pub struct TestClock {
    today: parking_lot::Mutex<NaiveDate>,
    now: parking_lot::Mutex<DateTime<Utc>>,
}

#[cfg(test)]
impl TestClock {
    pub fn new(date: NaiveDate) -> Self {
        let now = date
            .and_hms_opt(12, 0, 0)
            .map(|ndt| ndt.and_utc())
            .unwrap_or_else(Utc::now);
        Self {
            today: parking_lot::Mutex::new(date),
            now: parking_lot::Mutex::new(now),
        }
    }

    pub fn set(&self, date: NaiveDate) {
        *self.today.lock() = date;
        if let Some(ndt) = date.and_hms_opt(12, 0, 0) {
            *self.now.lock() = ndt.and_utc();
        }
    }
}

#[cfg(test)]
impl Clock for TestClock {
    fn today(&self) -> NaiveDate {
        *self.today.lock()
    }
    fn now(&self) -> DateTime<Utc> {
        *self.now.lock()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn entry(provider: Provider, cost: u64) -> UsageEntry {
        UsageEntry {
            timestamp: Utc::now(),
            provider,
            model: None,
            task: None,
            cost_cents: cost,
        }
    }

    #[tokio::test]
    async fn default_limits_daily_only() {
        let limits = BudgetLimits::default();
        assert_eq!(limits.daily_cents, Some(DEFAULT_DAILY_LIMIT_CENTS));
        assert_eq!(limits.session_cents, None);
    }

    #[tokio::test]
    async fn fresh_manager_has_ok_status_and_zero_spend() {
        let mgr = BudgetManager::with_defaults();
        let s = mgr.status().await;
        assert_eq!(s.state, BudgetState::Ok);
        assert_eq!(s.used_today_cents, 0);
        assert_eq!(s.used_session_cents, 0);
    }

    #[tokio::test]
    async fn record_accumulates_today_and_session() {
        let mgr = BudgetManager::with_defaults();
        mgr.record(entry(Provider::Fal, 100)).await;
        mgr.record(entry(Provider::Fal, 250)).await;
        let s = mgr.status().await;
        assert_eq!(s.used_today_cents, 350);
        assert_eq!(s.used_session_cents, 350);
    }

    #[tokio::test]
    async fn state_is_warn_between_80_and_100_percent() {
        let mgr = BudgetManager::new(BudgetLimits {
            daily_cents: Some(1000),
            session_cents: None,
        });
        mgr.record(entry(Provider::Fal, 800)).await;
        let s = mgr.status().await;
        assert_eq!(s.state, BudgetState::Warn);
    }

    #[tokio::test]
    async fn state_is_block_at_100_percent() {
        let mgr = BudgetManager::new(BudgetLimits {
            daily_cents: Some(500),
            session_cents: None,
        });
        mgr.record(entry(Provider::Fal, 500)).await;
        let s = mgr.status().await;
        assert_eq!(s.state, BudgetState::Block);
    }

    #[tokio::test]
    async fn would_block_is_true_when_projected_cost_pushes_over_limit() {
        let mgr = BudgetManager::new(BudgetLimits {
            daily_cents: Some(1000),
            session_cents: None,
        });
        mgr.record(entry(Provider::Fal, 900)).await;
        assert!(!mgr.would_block(99).await);
        assert!(mgr.would_block(200).await);
    }

    #[tokio::test]
    async fn session_limit_triggers_block_independently_of_daily() {
        let mgr = BudgetManager::new(BudgetLimits {
            daily_cents: Some(1_000_000),
            session_cents: Some(100),
        });
        mgr.record(entry(Provider::Fal, 100)).await;
        assert_eq!(mgr.status().await.state, BudgetState::Block);
    }

    #[tokio::test]
    async fn day_rollover_resets_daily_counter() {
        // Deterministic: inject a TestClock so day-boundary behaviour never
        // depends on wall-clock time. Previously this test used
        // `record_at`/`status_at`, which still worked fine for the math but
        // ran alongside a manager whose `day_started_at` was initialised from
        // `Utc::now()` — flaky when the test body crossed a UTC midnight.
        let day1 = NaiveDate::from_ymd_opt(2026, 4, 17).unwrap();
        let day2 = NaiveDate::from_ymd_opt(2026, 4, 18).unwrap();
        let clock = Arc::new(TestClock::new(day1));
        let mgr = BudgetManager::with_clock(
            BudgetLimits {
                daily_cents: Some(1000),
                session_cents: None,
            },
            clock.clone(),
        );

        mgr.record(entry(Provider::Fal, 500)).await;
        let before = mgr.status().await;
        assert_eq!(before.used_today_cents, 500);
        assert_eq!(before.used_session_cents, 500);

        // Advance to the next UTC day without touching the wall clock.
        clock.set(day2);
        let after = mgr.status().await;
        assert_eq!(
            after.used_today_cents, 0,
            "daily counter should have rolled"
        );
        // Session counter persists across day rollover.
        assert_eq!(after.used_session_cents, 500);
    }

    #[tokio::test]
    async fn start_new_session_resets_session_but_not_daily() {
        let mgr = BudgetManager::with_defaults();
        mgr.record(entry(Provider::Fal, 500)).await;
        mgr.start_new_session().await;
        let s = mgr.status().await;
        assert_eq!(s.used_session_cents, 0);
        assert_eq!(s.used_today_cents, 500);
    }

    #[tokio::test]
    async fn set_limits_overrides_defaults() {
        let mgr = BudgetManager::with_defaults();
        mgr.set_limits(BudgetLimits {
            daily_cents: Some(100),
            session_cents: Some(50),
        })
        .await;
        let s = mgr.status().await;
        assert_eq!(s.limits.daily_cents, Some(100));
        assert_eq!(s.limits.session_cents, Some(50));
    }

    #[tokio::test]
    async fn export_csv_has_header_and_one_row_per_entry() {
        let mgr = BudgetManager::with_defaults();
        mgr.record(UsageEntry {
            timestamp: Utc.with_ymd_and_hms(2026, 4, 17, 10, 0, 0).unwrap(),
            provider: Provider::Fal,
            model: Some(Model::FalFluxPro),
            task: Some(TaskKind::ImageGeneration),
            cost_cents: 3,
        })
        .await;
        let csv = mgr.export_csv().await;
        assert!(csv.starts_with("timestamp,provider,model,task,cost_cents\n"));
        // Provider serializes as kebab-case ("fal"), Model keeps PascalCase
        // ("FalFluxPro"), TaskKind is kebab-case ("image-generation").
        assert!(csv.contains("2026-04-17T10:00:00+00:00,fal,FalFluxPro,image-generation,3"));
    }

    #[test]
    fn cost_table_zero_for_abo_models_and_nonzero_for_paid() {
        // Claude Max is the only remaining truly-free abo — video abos now
        // carry non-zero rough-USD placeholders so daily budgets protect
        // against runaway spend against a near-empty credit balance.
        assert_eq!(cost_cents_for(Model::ClaudeOpus), 0);
        assert_eq!(cost_cents_for(Model::IdeogramV3), 0);
        assert!(cost_cents_for(Model::RunwayGen3) > 0);
        assert!(cost_cents_for(Model::HiggsfieldMulti) > 0);
        assert!(cost_cents_for(Model::ShotstackMontage) > 0);
        assert!(cost_cents_for(Model::FalFluxPro) > 0);
        assert!(cost_cents_for(Model::Kling20) > 0);
        assert!(cost_cents_for(Model::FalKlingV15) > 0);
        assert!(cost_cents_for(Model::FalKlingV2Master) > 0);
        assert!(cost_cents_for(Model::ReplicateFluxDev) > 0);
    }

    #[test]
    fn human_duration_format() {
        assert_eq!(human_duration(Duration::from_secs(59)), "0m");
        assert_eq!(human_duration(Duration::from_secs(180)), "3m");
        assert_eq!(human_duration(Duration::from_secs(3700)), "1h 1m");
    }

    #[tokio::test]
    async fn clear_resets_everything() {
        let mgr = BudgetManager::with_defaults();
        mgr.record(entry(Provider::Fal, 1000)).await;
        mgr.clear().await;
        let s = mgr.status().await;
        assert_eq!(s.used_today_cents, 0);
        assert_eq!(s.used_session_cents, 0);
        assert_eq!(mgr.entries().await.len(), 0);
    }

    #[test]
    fn zero_limit_is_treated_as_no_limit() {
        let state = compute_state(
            1_000_000,
            0,
            &BudgetLimits {
                daily_cents: Some(0),
                session_cents: None,
            },
        );
        assert_eq!(state, BudgetState::Ok);
    }
}
