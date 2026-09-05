use num_bigint::BigUint;
use reqwest::StatusCode;
use reqwest::blocking::Client;
use reqwest::header::{ACCEPT, HeaderMap, HeaderValue, USER_AGENT};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

pub(crate) use crate::tool::*;

pub(crate) const ROBINHOOD_CHAIN_ID: u64 = 4663;
const ROBINHOOD_API_BASE: &str = "https://api.robinhood.com/rhj";
const ROBINHOOD_RPC_URL: &str = "https://rpc.mainnet.chain.robinhood.com";
const BLOCKSCOUT_PUBLIC_BASE: &str = "https://robinhoodchain.blockscout.com/api/v2";
const BLOCKSCOUT_PRO_BASE: &str = "https://api.blockscout.com/4663/api/v2";
const ASSETS_CACHE_TTL: Duration = Duration::from_secs(300);
const QUOTES_CACHE_TTL: Duration = Duration::from_secs(15);
const CORPORATE_ACTIONS_CACHE_TTL: Duration = Duration::from_secs(3600);

type Timed<T> = (Instant, T);

static ASSETS_CACHE: OnceLock<Mutex<Option<Timed<Vec<Asset>>>>> = OnceLock::new();
static QUOTES_CACHE: OnceLock<Mutex<HashMap<String, Timed<Quote>>>> = OnceLock::new();
static CORPORATE_ACTIONS_CACHE: OnceLock<Mutex<Option<Timed<Vec<CorporateAction>>>>> =
    OnceLock::new();

#[derive(Clone, Default)]
pub(crate) struct HooditApp;

#[derive(Clone)]
pub(crate) struct HooditClient {
    http: Client,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Deployment {
    pub(crate) contract_address: String,
    pub(crate) chain_id: u64,
    #[serde(default)]
    pub(crate) network_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Asset {
    pub(crate) id: String,
    pub(crate) token_symbol: String,
    pub(crate) token_name: String,
    #[serde(default)]
    pub(crate) deployments: Vec<Deployment>,
    pub(crate) current_multiplier: String,
    #[serde(default)]
    pub(crate) pending_multiplier: String,
    #[serde(default)]
    pub(crate) pending_multiplier_effective_time: Option<String>,
    #[serde(default)]
    pub(crate) status: String,
    #[serde(default)]
    pub(crate) logo_url: Option<String>,
    #[serde(default)]
    pub(crate) trading_capabilities: Option<Value>,
    #[serde(default = "default_token_decimals")]
    pub(crate) token_decimals: u32,
    #[serde(default)]
    pub(crate) isin: Option<String>,
}

impl Asset {
    pub(crate) fn robinhood_chain_contract(&self) -> Option<&str> {
        self.deployments
            .iter()
            .find(|deployment| deployment.chain_id == ROBINHOOD_CHAIN_ID)
            .map(|deployment| deployment.contract_address.as_str())
    }

    pub(crate) fn is_active(&self) -> bool {
        self.status == "ASSET_STATUS_ACTIVE"
    }

    pub(crate) fn matches(&self, query: &str) -> bool {
        let query = query.trim().to_ascii_lowercase();
        if query.is_empty() {
            return true;
        }

        self.token_symbol.to_ascii_lowercase().contains(&query)
            || self.token_name.to_ascii_lowercase().contains(&query)
            || self
                .isin
                .as_deref()
                .is_some_and(|isin| isin.to_ascii_lowercase().contains(&query))
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct AssetsResponse {
    pub(crate) assets: Vec<Asset>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Quote {
    pub(crate) token_symbol: String,
    #[serde(default)]
    pub(crate) deployments: Vec<Deployment>,
    pub(crate) bid: String,
    pub(crate) ask: String,
    pub(crate) currency: String,
    #[serde(default)]
    pub(crate) daily_trading_volume: Option<String>,
    #[serde(default)]
    pub(crate) is_trading_halt: bool,
    pub(crate) generated_at: String,
    #[serde(default)]
    pub(crate) daily_high: Option<String>,
    #[serde(default)]
    pub(crate) daily_low: Option<String>,
    #[serde(default)]
    pub(crate) mint_burn_token_volume: Option<String>,
    #[serde(default)]
    pub(crate) mint_burn_usd_volume: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct QuotesResponse {
    pub(crate) quotes: Vec<Quote>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CorporateAction {
    pub(crate) id: String,
    #[serde(rename = "type")]
    pub(crate) action_type: String,
    pub(crate) status: String,
    #[serde(default)]
    pub(crate) process_date: Option<Value>,
    pub(crate) token_symbol: String,
    #[serde(default)]
    pub(crate) deployments: Vec<Deployment>,
    #[serde(default)]
    pub(crate) details: Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CorporateActionsResponse {
    pub(crate) corp_actions: Vec<CorporateAction>,
}

#[derive(Debug, Clone)]
pub(crate) struct IndexedBalance {
    pub(crate) contract_address: String,
    pub(crate) raw_value: String,
    pub(crate) decimals: Option<u32>,
}

#[derive(Debug)]
pub(crate) struct IndexedBalances {
    pub(crate) source: &'static str,
    pub(crate) balances: Vec<IndexedBalance>,
    pub(crate) fallback_reason: Option<String>,
}

impl HooditClient {
    pub(crate) fn new() -> Result<Self, String> {
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static("hoodit-app/0.1 (+https://github.com/aomi-labs/hoodit-apps)"),
        );
        let http = Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(20))
            .default_headers(headers)
            .build()
            .map_err(|error| format!("failed to initialize Hoodit HTTP client: {error}"))?;
        Ok(Self { http })
    }

    pub(crate) fn assets(&self) -> Result<Vec<Asset>, String> {
        let cache = ASSETS_CACHE.get_or_init(|| Mutex::new(None));
        if let Some((fetched_at, assets)) = cache.lock().map_err(cache_error)?.as_ref()
            && fetched_at.elapsed() < ASSETS_CACHE_TTL
        {
            return Ok(assets.clone());
        }

        let assets = self.get_robinhood::<AssetsResponse>("/assets")?.assets;
        *cache.lock().map_err(cache_error)? = Some((Instant::now(), assets.clone()));
        Ok(assets)
    }

    pub(crate) fn quote(&self, symbol: &str) -> Result<Quote, String> {
        let symbol = symbol.trim().to_ascii_uppercase();
        let cache = QUOTES_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
        if let Some((fetched_at, quote)) = cache.lock().map_err(cache_error)?.get(&symbol)
            && fetched_at.elapsed() < QUOTES_CACHE_TTL
        {
            return Ok(quote.clone());
        }

        let path = format!("/prices/{symbol}");
        let quote = self
            .get_robinhood::<QuotesResponse>(&path)?
            .quotes
            .into_iter()
            .next()
            .ok_or_else(|| format!("Robinhood returned no quote for {symbol}"))?;
        cache
            .lock()
            .map_err(cache_error)?
            .insert(symbol, (Instant::now(), quote.clone()));
        Ok(quote)
    }

    pub(crate) fn asset(&self, symbol: &str) -> Result<Asset, String> {
        self.assets()?
            .into_iter()
            .find(|asset| asset.token_symbol.eq_ignore_ascii_case(symbol))
            .ok_or_else(|| format!("No Robinhood Stock Token matched ticker {symbol}"))
    }

    pub(crate) fn quotes(&self) -> Result<Vec<Quote>, String> {
        let quotes = self.get_robinhood::<QuotesResponse>("/prices")?.quotes;
        let now = Instant::now();
        let mut cache = QUOTES_CACHE
            .get_or_init(|| Mutex::new(HashMap::new()))
            .lock()
            .map_err(cache_error)?;
        for quote in &quotes {
            cache.insert(
                quote.token_symbol.to_ascii_uppercase(),
                (now, quote.clone()),
            );
        }
        Ok(quotes)
    }

    pub(crate) fn corporate_actions(&self) -> Result<Vec<CorporateAction>, String> {
        let cache = CORPORATE_ACTIONS_CACHE.get_or_init(|| Mutex::new(None));
        if let Some((fetched_at, actions)) = cache.lock().map_err(cache_error)?.as_ref()
            && fetched_at.elapsed() < CORPORATE_ACTIONS_CACHE_TTL
        {
            return Ok(actions.clone());
        }

        let actions = self
            .get_robinhood::<CorporateActionsResponse>("/corporate-actions")?
            .corp_actions;
        *cache.lock().map_err(cache_error)? = Some((Instant::now(), actions.clone()));
        Ok(actions)
    }

    pub(crate) fn token_balance(&self, contract: &str, address: &str) -> Result<String, String> {
        let calldata = format!("0x70a08231{:0>64}", &address[2..].to_ascii_lowercase());
        let request = self.http.post(ROBINHOOD_RPC_URL).json(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eth_call",
            "params": [{ "to": contract, "data": calldata }, "latest"]
        }));
        let value = self.send_json(request, "Robinhood Chain RPC")?;
        if let Some(error) = value.get("error") {
            return Err(format!(
                "Robinhood Chain RPC returned an error: {}",
                concise_body(&error.to_string())
            ));
        }
        let hex_value = value
            .get("result")
            .and_then(Value::as_str)
            .and_then(|value| value.strip_prefix("0x"))
            .ok_or_else(|| "Robinhood Chain RPC returned no token balance".to_string())?;
        BigUint::parse_bytes(hex_value.as_bytes(), 16)
            .map(|value| value.to_str_radix(10))
            .ok_or_else(|| "Robinhood Chain RPC returned an invalid token balance".to_string())
    }

    pub(crate) fn indexed_balances(
        &self,
        address: &str,
        alchemy_api_key: Option<&str>,
        blockscout_api_key: Option<&str>,
    ) -> Result<IndexedBalances, String> {
        if let Some(api_key) = alchemy_api_key {
            match self.alchemy_balances(address, api_key) {
                Ok(balances) => {
                    return Ok(IndexedBalances {
                        source: "Alchemy Data API",
                        balances,
                        fallback_reason: None,
                    });
                }
                Err(alchemy_error) => {
                    return self
                        .blockscout_balances(address, blockscout_api_key)
                        .map(|balances| IndexedBalances {
                            source: "Blockscout",
                            balances,
                            fallback_reason: Some(alchemy_error),
                        });
                }
            }
        }

        self.blockscout_balances(address, blockscout_api_key)
            .map(|balances| IndexedBalances {
                source: "Blockscout",
                balances,
                fallback_reason: None,
            })
    }

    fn alchemy_balances(
        &self,
        address: &str,
        api_key: &str,
    ) -> Result<Vec<IndexedBalance>, String> {
        let request = self
            .http
            .post(format!(
                "https://robinhood-mainnet.g.alchemy.com/v2/{api_key}"
            ))
            .json(&json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "alchemy_getTokenBalances",
                "params": [address, "erc20"]
            }));
        let value = self.send_json(request, "Alchemy portfolio")?;
        if let Some(error) = value.get("error") {
            return Err(format!(
                "Alchemy portfolio returned an RPC error: {}",
                concise_body(&error.to_string())
            ));
        }
        let balances = value
            .pointer("/result/tokenBalances")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                "Alchemy portfolio response did not contain token balances".to_string()
            })?;
        Ok(balances.iter().filter_map(alchemy_balance).collect())
    }

    fn blockscout_balances(
        &self,
        address: &str,
        blockscout_api_key: Option<&str>,
    ) -> Result<Vec<IndexedBalance>, String> {
        let mut request = if let Some(api_key) = blockscout_api_key {
            self.http
                .get(format!(
                    "{BLOCKSCOUT_PRO_BASE}/addresses/{address}/token-balances"
                ))
                .query(&[("apikey", api_key)])
        } else {
            self.http.get(format!(
                "{BLOCKSCOUT_PUBLIC_BASE}/addresses/{address}/token-balances"
            ))
        };
        request = request.header(ACCEPT, "application/json");

        let value = self.send_json(request, "Blockscout portfolio")?;
        let balances = value
            .as_array()
            .ok_or_else(|| "Blockscout portfolio response was not an array".to_string())?;
        Ok(balances.iter().filter_map(indexed_balance).collect())
    }

    fn get_robinhood<T>(&self, path: &str) -> Result<T, String>
    where
        T: for<'de> Deserialize<'de>,
    {
        let request = self.http.get(format!("{ROBINHOOD_API_BASE}{path}"));
        let value = self.send_json(request, "Robinhood Stock Token API")?;
        serde_json::from_value(value)
            .map_err(|error| format!("Robinhood response shape changed: {error}"))
    }

    fn send_json(
        &self,
        request: reqwest::blocking::RequestBuilder,
        source: &str,
    ) -> Result<Value, String> {
        for attempt in 0..3 {
            let response = request
                .try_clone()
                .ok_or_else(|| format!("{source} request could not be retried"))?
                .send()
                .map_err(|_| format!("{source} request failed"))?;
            let status = response.status();
            if status == StatusCode::TOO_MANY_REQUESTS && attempt < 2 {
                let retry_after = response
                    .headers()
                    .get("retry-after")
                    .and_then(|value| value.to_str().ok())
                    .and_then(|value| value.parse::<u64>().ok())
                    .unwrap_or(attempt + 1)
                    .clamp(1, 2);
                thread::sleep(Duration::from_secs(retry_after));
                continue;
            }

            let body = response
                .text()
                .map_err(|error| format!("{source} response could not be read: {error}"))?;
            if !status.is_success() {
                if status == StatusCode::TOO_MANY_REQUESTS {
                    return Err(format!("{source} rate limited the request; retry shortly"));
                }
                return Err(format!(
                    "{source} returned HTTP {status}: {}",
                    concise_body(&body)
                ));
            }

            return serde_json::from_str(&body)
                .map_err(|error| format!("{source} returned invalid JSON: {error}"));
        }
        Err(format!("{source} rate limited the request; retry shortly"))
    }
}

pub(crate) fn assets_by_contract(assets: &[Asset]) -> HashMap<String, &Asset> {
    assets
        .iter()
        .filter_map(|asset| {
            asset
                .robinhood_chain_contract()
                .map(|address| (address.to_ascii_lowercase(), asset))
        })
        .collect()
}

pub(crate) fn quotes_by_symbol(quotes: &[Quote]) -> HashMap<String, &Quote> {
    quotes
        .iter()
        .map(|quote| (quote.token_symbol.to_ascii_uppercase(), quote))
        .collect()
}

pub(crate) fn decimal_product(values: &[&str]) -> Option<String> {
    values
        .iter()
        .try_fold(Decimal::ONE, |product, value| {
            Decimal::from_str(value)
                .ok()
                .and_then(|value| product.checked_mul(value))
        })
        .map(normalize_decimal)
}

pub(crate) fn decimal_midpoint(bid: &str, ask: &str) -> Option<String> {
    let bid = Decimal::from_str(bid).ok()?;
    let ask = Decimal::from_str(ask).ok()?;
    bid.checked_add(ask)
        .and_then(|sum| sum.checked_div(Decimal::from(2)))
        .map(normalize_decimal)
}

pub(crate) fn format_units(raw: &str, decimals: u32) -> Option<String> {
    if raw.is_empty() || !raw.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let trimmed = raw.trim_start_matches('0');
    let digits = if trimmed.is_empty() { "0" } else { trimmed };
    if decimals == 0 {
        return Some(digits.to_string());
    }

    let decimals = usize::try_from(decimals).ok()?;
    let padded = if digits.len() <= decimals {
        format!("{}{}", "0".repeat(decimals + 1 - digits.len()), digits)
    } else {
        digits.to_string()
    };
    let split = padded.len() - decimals;
    let whole = &padded[..split];
    let fraction = padded[split..].trim_end_matches('0');
    Some(if fraction.is_empty() {
        whole.to_string()
    } else {
        format!("{whole}.{fraction}")
    })
}

fn indexed_balance(value: &Value) -> Option<IndexedBalance> {
    let token = value.get("token")?;
    let contract_address = token
        .get("address_hash")
        .or_else(|| token.get("address"))?
        .as_str()?
        .to_string();
    let raw_value = value.get("value")?.as_str()?.to_string();
    let decimals = token.get("decimals").and_then(|value| {
        value
            .as_u64()
            .or_else(|| value.as_str()?.parse::<u64>().ok())
            .and_then(|value| u32::try_from(value).ok())
    });
    Some(IndexedBalance {
        contract_address,
        raw_value,
        decimals,
    })
}

fn alchemy_balance(value: &Value) -> Option<IndexedBalance> {
    let contract_address = value.get("contractAddress")?.as_str()?.to_string();
    let hex_value = value.get("tokenBalance")?.as_str()?.strip_prefix("0x")?;
    let raw_value = BigUint::parse_bytes(hex_value.as_bytes(), 16)?.to_str_radix(10);
    Some(IndexedBalance {
        contract_address,
        raw_value,
        decimals: None,
    })
}

fn normalize_decimal(value: Decimal) -> String {
    value.normalize().to_string()
}

fn default_token_decimals() -> u32 {
    18
}

fn concise_body(body: &str) -> String {
    const MAX_CHARS: usize = 240;
    let body = body.split_whitespace().collect::<Vec<_>>().join(" ");
    if body.chars().count() <= MAX_CHARS {
        return body;
    }
    format!("{}…", body.chars().take(MAX_CHARS).collect::<String>())
}

fn cache_error<T>(_error: std::sync::PoisonError<T>) -> String {
    "Hoodit response cache is unavailable".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn formats_large_erc20_values_without_float_loss() {
        assert_eq!(
            format_units("1234500000000000000", 18).as_deref(),
            Some("1.2345")
        );
        assert_eq!(
            format_units("42", 18).as_deref(),
            Some("0.000000000000000042")
        );
        assert_eq!(format_units("000", 18).as_deref(), Some("0"));
        assert_eq!(format_units("not-a-number", 18), None);
    }

    #[test]
    fn applies_multiplier_to_reference_price() {
        assert_eq!(
            decimal_product(&["320.10", "0.25"]).as_deref(),
            Some("80.025")
        );
        assert_eq!(decimal_midpoint("300", "320.1").as_deref(), Some("310.05"));
    }

    #[test]
    fn reads_blockscout_balance_shape() {
        let value = json!({
            "token": {
                "address_hash": "0xaF3D76f1834A1d425780943C99Ea8A608f8a93f9",
                "decimals": "18"
            },
            "value": "1500000000000000000"
        });
        let balance = indexed_balance(&value).expect("valid balance");
        assert_eq!(balance.decimals, Some(18));
        assert_eq!(balance.raw_value, "1500000000000000000");
    }

    #[test]
    fn reads_alchemy_balance_shape() {
        let value = json!({
            "contractAddress": "0xaF3D76f1834A1d425780943C99Ea8A608f8a93f9",
            "tokenBalance": "0x14d1120d7b160000"
        });
        let balance = alchemy_balance(&value).expect("valid balance");
        assert_eq!(balance.decimals, None);
        assert_eq!(balance.raw_value, "1500000000000000000");
    }
}
