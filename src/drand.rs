use curl::easy::Easy;
use serde::Deserialize;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const GENESIS_TIME: u64 = 1595431050;
pub const PERIOD: u64 = 30;

const DRAND_BASE: &str = "https://api.drand.sh/public";
const TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Deserialize)]
pub struct DrandResponse {
    pub round: u64,
    pub randomness: String,
}

pub fn now_secs() -> u64 {
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

    let mut body = Vec::new();
    let mut handle = Easy::new();
    handle.url(&url).map_err(|e| format!("curl error: {e}"))?;
    handle
        .timeout(TIMEOUT)
        .map_err(|e| format!("curl error: {e}"))?;

    {
        let mut transfer = handle.transfer();
        transfer
            .write_function(|data| {
                body.extend_from_slice(data);
                Ok(data.len())
            })
            .map_err(|e| format!("curl error: {e}"))?;
        transfer
            .perform()
            .map_err(|e| format!("drand request failed: {e}"))?;
    }

    let code = handle.response_code().unwrap_or(0);
    match code {
        200 => {}
        425 => {
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
                return Err(format!("round {r} is in the future (available {wait})"));
            }
            return Err(format!("drand request failed: HTTP {code}"));
        }
        500 => {
            if let Some(r) = round {
                let available_at = GENESIS_TIME + r * PERIOD;
                let now = now_secs();
                if available_at.saturating_sub(now) <= PERIOD
                    || now.saturating_sub(available_at) <= PERIOD
                {
                    return Err(format!(
                        "round {r} is not yet available -- it is being generated, try again in a moment"
                    ));
                }
            }
            return Err(format!("drand request failed: HTTP {code}"));
        }
        _ => return Err(format!("drand request failed: HTTP {code}")),
    }

    let text = String::from_utf8(body).map_err(|e| format!("failed to read response: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("failed to parse drand response: {e}"))
}
