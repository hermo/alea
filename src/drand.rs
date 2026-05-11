use serde::Deserialize;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use ureq::{
    Agent,
    config::Config,
    tls::{TlsConfig, TlsProvider},
};

pub const GENESIS_TIME: u64 = 1595431050;
pub const PERIOD: u64 = 30;

const DRAND_BASE: &str = "https://api.drand.sh/public";
const TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Deserialize)]
pub struct DrandResponse {
    pub round: u64,
    pub randomness: String,
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn fetch(round: Option<u64>) -> Result<DrandResponse, String> {
    let url = match round {
        Some(r) => format!("{DRAND_BASE}/{r}"),
        None => format!("{DRAND_BASE}/latest"),
    };

    let agent: Agent = Config::builder()
        .tls_config(
            TlsConfig::builder()
                .provider(TlsProvider::NativeTls)
                .build(),
        )
        .timeout_global(Some(TIMEOUT))
        .build()
        .into();

    let body = agent
        .get(&url)
        .call()
        .map_err(|e| {
            match e {
                ureq::Error::StatusCode(425) => {
                    if let Some(r) = round {
                        let available_at = GENESIS_TIME + r * PERIOD;
                        let secs = available_at.saturating_sub(now_secs());
                        let wait = if secs >= 3600 {
                            format!("in ~{} h {} min", secs / 3600, (secs % 3600) / 60)
                        } else if secs >= 60 {
                            format!("in ~{} min", secs / 60)
                        } else {
                            format!("in ~{secs} s")
                        };
                        return format!("round {r} is in the future (available {wait})");
                    }
                    format!("drand request failed: {e}")
                }
                ureq::Error::StatusCode(500) => {
                    if let Some(r) = round {
                        let available_at = GENESIS_TIME + r * PERIOD;
                        let now = now_secs();
                        // 500 near the chain tip means the round is being generated
                        if available_at.saturating_sub(now) <= PERIOD
                            || now.saturating_sub(available_at) <= PERIOD
                        {
                            return format!(
                                "round {r} is not yet available — it is being generated, try again in a moment"
                            );
                        }
                    }
                    format!("drand request failed: {e}")
                }
                ureq::Error::Timeout(_) => {
                    if let Some(r) = round {
                        let available_at = GENESIS_TIME + r * PERIOD;
                        let now = now_secs();
                        if available_at.saturating_sub(now) <= PERIOD
                            || now.saturating_sub(available_at) <= PERIOD
                        {
                            return format!(
                                "round {r} is not yet available — it is being generated, try again in a moment"
                            );
                        }
                    }
                    "drand request failed: timed out".to_string()
                }
                _ => format!("drand request failed: {e}"),
            }
        })?
        .into_body()
        .read_to_string()
        .map_err(|e| format!("failed to read response: {e}"))?;

    serde_json::from_str(&body).map_err(|e| format!("failed to parse drand response: {e}"))
}
