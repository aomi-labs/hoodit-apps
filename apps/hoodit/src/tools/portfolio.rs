use super::{present, provider_error};
use crate::{
    amount,
    app::{HooditApp, ReadContext},
    model,
    providers::{Blockscout, Lifi, ProviderError},
};
use aomi_sdk::schemars::JsonSchema;
use aomi_sdk::{DynAomiTool, DynToolCallCtx};
use chrono::Utc;
use num_traits::Zero;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PortfolioArgs {
    pub wallet_address: String,
    /// Opaque continuation returned by the previous portfolio response. Omit
    /// this field entirely for the first page; never send an empty value or
    /// the string "null".
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(
        with = "String",
        length(min = 1, max = 4096),
        pattern(r"^[A-Za-z0-9_-]+$")
    )]
    pub cursor: Option<String>,
    #[serde(default, deserialize_with = "present")]
    #[schemars(with = "bool", extend("default" = false))]
    pub include_quotes: Option<bool>,
    #[serde(default, deserialize_with = "present")]
    #[schemars(with = "bool", extend("default" = false))]
    pub refresh: Option<bool>,
}
pub struct GetPortfolio;
impl DynAomiTool for GetPortfolio {
    type App = HooditApp;
    type Args = PortfolioArgs;
    const NAME: &'static str = "hoodit_get_portfolio";
    const DESCRIPTION: &'static str = "Read one Blockscout wallet inventory page and optional bounded LI.FI sample valuations. Omit cursor entirely for the first page; only reuse a next_cursor returned by an earlier response for the same wallet.";
    fn run(app: &HooditApp, args: PortfolioArgs, ctx: DynToolCallCtx) -> Result<Value, String> {
        let mut read = ReadContext::portfolio(args.refresh.unwrap_or(false));
        let wallet = match model::address(&args.wallet_address) {
            Ok(value) => value,
            Err(message) => return Ok(model::error("INVALID_ARGUMENT", &message, false)),
        };
        let runtime = app.runtime()?;
        let block = match Blockscout::from_ctx(&runtime, &ctx) {
            Ok(v) => v,
            Err(e) => return Ok(provider_error(e)),
        };
        let inventory = match block.inventory(&wallet, args.cursor.as_deref(), &mut read) {
            Ok(v) => v,
            Err(e) => return Ok(provider_error(e)),
        };
        let include = args.include_quotes.unwrap_or(false);
        if include && let Err(e) = verify_usdg(&block, &mut read) {
            return Ok(provider_error(e));
        }
        let lifi = Lifi::from_ctx(&runtime, &ctx);
        let mut holdings = vec![];
        let mut warnings = vec![];
        let mut quote_attempts = 0;
        let mut seen = std::collections::HashSet::new();
        let native_initial = args.cursor.is_none();
        let native_raw = if native_initial {
            match block.native_balance(&wallet, &mut read) {
                Ok(raw) => Some(raw),
                Err(_) => {
                    warnings.push(model::warning(
                        "NATIVE_BALANCE_UNAVAILABLE",
                        "Native balance could not be established",
                    ));
                    None
                }
            }
        } else {
            None
        };
        let native_included = native_raw.is_some();
        let mut returned = 0;
        for item in &inventory.items {
            match inventory_holding(
                item,
                &wallet,
                include,
                &mut quote_attempts,
                &lifi,
                &mut read,
                &mut warnings,
            ) {
                Ok(Some(h)) => {
                    let id = model::string(&h, &["token", "id"]).unwrap_or_default();
                    if !seen.insert(id) {
                        return Ok(provider_error(ProviderError {
                            code: "UPSTREAM_SCHEMA_CHANGED",
                            message: "Blockscout returned a duplicate token holding".into(),
                            retryable: false,
                        }));
                    }
                    returned += 1;
                    holdings.push(h);
                }
                Ok(None) => {}
                Err(e) => return Ok(provider_error(e)),
            }
        }
        if let Some(raw) = native_raw
            && raw != "0"
        {
            match make_holding(
                "native",
                Some("ETH"),
                Some("Ether"),
                Some(18),
                &raw,
                &wallet,
                include,
                100,
                &mut quote_attempts,
                &lifi,
                &mut read,
                &mut warnings,
            ) {
                Ok(holding) => holdings.push(holding),
                Err(e) => return Ok(provider_error(e)),
            }
        }
        sort_holdings(&mut holdings);
        let scope = if inventory.next_cursor.is_none() && native_included {
            "wallet"
        } else {
            warnings.push(model::warning(
                "PARTIAL_PAGE",
                "This response does not cover a complete wallet traversal",
            ));
            "page"
        };
        let priced = holdings
            .iter()
            .filter(|h| h["valuation"]["value_usdg"].is_string())
            .count();
        let unpriced = holdings.len() - priced;
        let priced_total = sum_values(&holdings);
        let total = if scope == "wallet" && include && unpriced == 0 {
            priced_total.clone().or_else(|| Some("0".into()))
        } else {
            None
        };
        Ok(model::ok(
            json!({"wallet_address":wallet,"quote_token":quote_token(include),"native_included":native_included,"holdings":holdings,"pagination":{"returned":returned,"next_cursor":inventory.next_cursor},"summary":{"scope":scope,"holdings_count":priced+unpriced,"priced_count":priced,"unpriced_count":unpriced,"priced_value_usdg":if include {priced_total}else{None},"total_value_usdg":total}}),
            read.sources,
            {
                warnings.extend(read.warnings);
                warnings
            },
        ))
    }
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HoldingArgs {
    pub wallet_address: String,
    pub token: String,
    #[serde(default, deserialize_with = "present")]
    #[schemars(with = "u16", range(min = 1, max = 10000), extend("default" = 100))]
    pub quote_balance_bps: Option<u16>,
    #[serde(default, deserialize_with = "present")]
    #[schemars(with = "bool", extend("default" = true))]
    pub include_quote: Option<bool>,
    #[serde(default, deserialize_with = "present")]
    #[schemars(with = "bool", extend("default" = true))]
    pub refresh: Option<bool>,
}
pub struct GetHolding;
impl DynAomiTool for GetHolding {
    type App = HooditApp;
    type Args = HoldingArgs;
    const NAME: &'static str = "hoodit_get_holding";
    const DESCRIPTION: &'static str = "Read an exact wallet holding, floor-size a requested fraction, and optionally estimate USDG proceeds.";
    fn run(app: &HooditApp, args: HoldingArgs, ctx: DynToolCallCtx) -> Result<Value, String> {
        let mut read = ReadContext::portfolio(args.refresh.unwrap_or(true));
        let wallet = match model::address(&args.wallet_address) {
            Ok(value) => value,
            Err(message) => return Ok(model::error("INVALID_ARGUMENT", &message, false)),
        };
        let token = match model::token_id(&args.token) {
            Ok(value) => value,
            Err(message) => return Ok(model::error("INVALID_ARGUMENT", &message, false)),
        };
        let bps = args.quote_balance_bps.unwrap_or(100);
        if !(1..=10_000).contains(&bps) {
            return Ok(model::error(
                "INVALID_ARGUMENT",
                "quote_balance_bps must be 1 to 10000",
                false,
            ));
        }
        let runtime = app.runtime()?;
        let block = match Blockscout::from_ctx(&runtime, &ctx) {
            Ok(v) => v,
            Err(e) => return Ok(provider_error(e)),
        };
        if args.include_quote.unwrap_or(true)
            && let Err(e) = verify_usdg(&block, &mut read)
        {
            return Ok(provider_error(e));
        }
        let (raw, symbol, name, decimals) = if token == "native" {
            match block.native_balance(&wallet, &mut read) {
                Ok(v) => (v, Some("ETH".into()), Some("Ether".into()), Some(18)),
                Err(e) => return Ok(provider_error(e)),
            }
        } else {
            let raw = match block.token_balance(&wallet, &token, &mut read) {
                Ok(v) => v,
                Err(e) => return Ok(provider_error(e)),
            };
            let mut info = block.token_info(&token, &mut read).ok();
            if let Some(value) = info.as_ref() {
                match model::string(value, &["address_hash"]) {
                    Some(address) if address.eq_ignore_ascii_case(&token) => {}
                    Some(_) => {
                        return Ok(provider_error(ProviderError {
                            code: "UPSTREAM_SCHEMA_CHANGED",
                            message:
                                "Blockscout token metadata identity does not match the request"
                                    .into(),
                            retryable: false,
                        }));
                    }
                    None => info = None,
                }
            }
            (
                raw,
                info.as_ref().and_then(|v| model::string(v, &["symbol"])),
                info.as_ref().and_then(|v| model::string(v, &["name"])),
                info.as_ref()
                    .and_then(|v| model::string(v, &["decimals"]))
                    .and_then(|v| v.parse().ok()),
            )
        };
        let balance = match amount::atomic(&raw) {
            Ok(value) => value,
            Err(_) => {
                return Ok(model::error(
                    "UPSTREAM_SCHEMA_CHANGED",
                    "provider returned an invalid balance",
                    false,
                ));
            }
        };
        let sell = amount::fraction(&balance, bps);
        let mut warnings = vec![];
        let lifi = Lifi::from_ctx(&runtime, &ctx);
        let mut quote_attempts = 0;
        let holding = match make_holding(
            &token,
            symbol.as_deref(),
            name.as_deref(),
            decimals,
            &raw,
            &wallet,
            args.include_quote.unwrap_or(true),
            bps,
            &mut quote_attempts,
            &lifi,
            &mut read,
            &mut warnings,
        ) {
            Ok(holding) => holding,
            Err(e) => return Ok(provider_error(e)),
        };
        let sell_formatted = decimals.map(|d| amount::format(&sell, d));
        Ok(model::ok(
            json!({"wallet_address":wallet,"quote_token":quote_token(args.include_quote.unwrap_or(true)),"holding":holding,"requested_balance_bps":bps,"sell_amount":{"atomic":sell.to_string(),"formatted":sell_formatted}}),
            read.sources,
            {
                warnings.extend(read.warnings);
                warnings
            },
        ))
    }
}

fn inventory_holding(
    item: &Value,
    wallet: &str,
    include: bool,
    quote_attempts: &mut usize,
    lifi: &Lifi,
    read: &mut ReadContext,
    warnings: &mut Vec<Value>,
) -> Result<Option<Value>, ProviderError> {
    let invalid = || ProviderError {
        code: "UPSTREAM_SCHEMA_CHANGED",
        message: "Blockscout returned an invalid inventory holding".into(),
        retryable: false,
    };
    let raw = model::string(item, &["value"]).ok_or_else(invalid)?;
    let atomic = amount::atomic(&raw).map_err(|_| invalid())?;
    if atomic.is_zero() {
        return Ok(None);
    }
    let t = item.get("token").ok_or_else(invalid)?;
    let id = model::string(t, &["address_hash"])
        .ok_or_else(invalid)
        .and_then(|id| model::address(&id).map_err(|_| invalid()))?;
    make_holding(
        &id,
        model::string(t, &["symbol"]).as_deref(),
        model::string(t, &["name"]).as_deref(),
        model::string(t, &["decimals"]).and_then(|v| v.parse().ok()),
        &raw,
        wallet,
        include,
        100,
        quote_attempts,
        lifi,
        read,
        warnings,
    )
    .map(Some)
}
fn verify_usdg(block: &Blockscout, read: &mut ReadContext) -> Result<(), ProviderError> {
    let info = block.token_info(model::USDG, read)?;
    let address =
        model::string(&info, &["address_hash"]).and_then(|value| model::address(&value).ok());
    let decimals = model::string(&info, &["decimals"]).and_then(|value| value.parse::<u8>().ok());
    if address
        .as_deref()
        .is_some_and(|value| value.eq_ignore_ascii_case(model::USDG))
        && decimals == Some(6)
    {
        Ok(())
    } else {
        Err(ProviderError {
            code: "UPSTREAM_SCHEMA_CHANGED",
            message: "USDG identity or decimals could not be verified".into(),
            retryable: false,
        })
    }
}
fn quote_token(verified: bool) -> Value {
    model::token(
        model::USDG,
        verified.then_some("USDG"),
        verified.then_some("Global Dollar"),
        verified.then_some(6),
        None,
    )
}
fn sort_holdings(holdings: &mut [Value]) {
    use std::cmp::Ordering;
    use std::str::FromStr;
    holdings.sort_by(|left, right| {
        let value = |holding: &Value| {
            model::string(holding, &["valuation", "value_usdg"])
                .and_then(|value| bigdecimal::BigDecimal::from_str(&value).ok())
        };
        match (value(left), value(right)) {
            (Some(left), Some(right)) => right.partial_cmp(&left).unwrap_or(Ordering::Equal),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        }
        .then_with(|| {
            model::string(left, &["token", "id"]).cmp(&model::string(right, &["token", "id"]))
        })
    });
}
fn sum_values(holdings: &[Value]) -> Option<String> {
    use std::str::FromStr;
    let values = holdings
        .iter()
        .filter_map(|h| model::string(h, &["valuation", "value_usdg"]))
        .filter_map(|v| bigdecimal::BigDecimal::from_str(&v).ok())
        .collect::<Vec<_>>();
    if values.is_empty() {
        None
    } else {
        Some(
            values
                .into_iter()
                .sum::<bigdecimal::BigDecimal>()
                .normalized()
                .to_plain_string(),
        )
    }
}
#[allow(clippy::too_many_arguments)]
fn make_holding(
    id: &str,
    symbol: Option<&str>,
    name: Option<&str>,
    decimals: Option<u8>,
    raw: &str,
    wallet: &str,
    include: bool,
    sample_bps: u16,
    quote_attempts: &mut usize,
    lifi: &Lifi,
    read: &mut ReadContext,
    warnings: &mut Vec<Value>,
) -> Result<Value, ProviderError> {
    let balance = amount::atomic(raw).map_err(|_| ProviderError {
        code: "UPSTREAM_SCHEMA_CHANGED",
        message: "provider returned an invalid balance".into(),
        retryable: false,
    })?;
    let formatted = decimals.map(|d| amount::format(&balance, d));
    let valuation = if balance.is_zero() {
        json!({"status":"zero_balance","unit_price_usdg":null,"value_usdg":"0","reason":null,"quote":null})
    } else if decimals.is_none() {
        warnings.push(model::warning(
            "UNKNOWN_DECIMALS",
            "Token decimals are unavailable; the raw balance is preserved",
        ));
        json!({"status":"unpriced","unit_price_usdg":null,"value_usdg":null,"reason":"unknown_decimals","quote":null})
    } else if !include {
        json!({"status":"not_requested","unit_price_usdg":null,"value_usdg":null,"reason":null,"quote":null})
    } else if id.eq_ignore_ascii_case(model::USDG) {
        json!({"status":"quote_currency","unit_price_usdg":"1","value_usdg":formatted,"reason":null,"quote":null})
    } else {
        let sample = amount::fraction(&balance, sample_bps);
        if sample.is_zero() {
            warnings.push(model::warning(
                "QUOTE_SIZE_ROUNDS_TO_ZERO",
                "Requested fraction rounds down to zero atomic units",
            ));
            json!({"status":"unpriced","unit_price_usdg":null,"value_usdg":null,"reason":"rounds_to_zero","quote":null})
        } else if *quote_attempts >= 20 {
            warnings.push(model::warning(
                "QUOTE_BUDGET_EXHAUSTED",
                "At most twenty non-USDG quotes are attempted per response",
            ));
            json!({"status":"unpriced","unit_price_usdg":null,"value_usdg":null,"reason":"budget_exhausted","quote":null})
        } else {
            *quote_attempts += 1;
            let from = if id == "native" {
                model::NATIVE_SENTINEL
            } else {
                id
            };
            match lifi.quote(wallet, from, &sample.to_string(), read) {
                Ok(q) => match quote_amounts(&q, decimals.unwrap()) {
                    Some((out, min)) => {
                        let (unit, value) =
                            amount::extrapolate(&balance, &sample, &out, decimals.unwrap(), 6)
                                .unwrap();
                        json!({"status":"quoted","unit_price_usdg":unit,"value_usdg":value,"reason":null,"quote":{"input_amount":{"atomic":sample.to_string(),"formatted":decimals.map(|d|amount::format(&sample,d))},"expected_output":{"atomic":out.to_string(),"formatted":amount::format(&out,6)},"minimum_output":{"atomic":min.to_string(),"formatted":amount::format(&min,6)},"route_name":model::string(&q,&["tool"]),"gas_cost_usd":sum_gas_costs(&q),"quoted_at":Utc::now().to_rfc3339(),"slippage_bps":50,"preflighted":false}})
                    }
                    _ => {
                        warnings.push(model::warning(
                            "QUOTE_UNAVAILABLE",
                            "LI.FI returned an inconsistent quote",
                        ));
                        json!({"status":"unpriced","unit_price_usdg":null,"value_usdg":null,"reason":"quote_unavailable","quote":null})
                    }
                },
                Err(e) => {
                    let reason = if e.code == "RATE_LIMITED" {
                        "rate_limited"
                    } else if e.code == "NOT_INDEXED" {
                        "no_route"
                    } else if e.code == "QUOTE_BUDGET_EXHAUSTED" {
                        "budget_exhausted"
                    } else if e.code == "DEADLINE_EXCEEDED" {
                        "deadline_exceeded"
                    } else {
                        "quote_unavailable"
                    };
                    warnings.push(model::warning(
                        match reason {
                            "rate_limited" => "RATE_LIMITED",
                            "budget_exhausted" => "QUOTE_BUDGET_EXHAUSTED",
                            "deadline_exceeded" => "DEADLINE_EXCEEDED",
                            _ => "QUOTE_UNAVAILABLE",
                        },
                        "A requested holding valuation was unavailable",
                    ));
                    json!({"status":"unpriced","unit_price_usdg":null,"value_usdg":null,"reason":reason,"quote":null})
                }
            }
        }
    };
    Ok(
        json!({"token":model::token(id,symbol,name,decimals,None),"balance":{"atomic":raw,"formatted":formatted},"valuation":valuation}),
    )
}

fn quote_amounts(
    q: &Value,
    input_decimals: u8,
) -> Option<(num_bigint::BigUint, num_bigint::BigUint)> {
    let decimal = |path: &[&str]| {
        model::get(q, path)
            .and_then(|value| value.as_u64().or_else(|| value.as_str()?.parse().ok()))
    };
    if decimal(&["action", "fromToken", "decimals"]) != Some(input_decimals.into())
        || decimal(&["action", "toToken", "decimals"]) != Some(6)
    {
        return None;
    }
    let out = model::string(q, &["estimate", "toAmount"])
        .and_then(|value| amount::atomic(&value).ok())?;
    let min = model::string(q, &["estimate", "toAmountMin"])
        .and_then(|value| amount::atomic(&value).ok())?;
    (min <= out).then_some((out, min))
}

fn sum_gas_costs(q: &Value) -> Option<String> {
    use std::str::FromStr;
    let values = model::get(q, &["estimate", "gasCosts"])
        .and_then(Value::as_array)?
        .iter()
        .map(|cost| {
            model::string(cost, &["amountUSD"])
                .and_then(|value| bigdecimal::BigDecimal::from_str(&value).ok())
        })
        .collect::<Option<Vec<_>>>()?;
    (!values.is_empty()).then(|| {
        values
            .into_iter()
            .sum::<bigdecimal::BigDecimal>()
            .normalized()
            .to_plain_string()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{ProviderOrigins, Runtime};
    use reqwest::blocking::Client;
    use std::collections::HashMap;

    fn fixture() -> (Runtime, DynToolCallCtx) {
        (
            Runtime::fixture(Client::new(), ProviderOrigins::default()),
            DynToolCallCtx {
                session_id: "test".into(),
                tool_name: "portfolio".into(),
                call_id: "1".into(),
                state_attributes: Default::default(),
                secrets: HashMap::new(),
            },
        )
    }

    #[test]
    fn malformed_inventory_balance_is_an_upstream_error() {
        let (runtime, ctx) = fixture();
        let lifi = Lifi::from_ctx(&runtime, &ctx);
        let mut read = ReadContext::portfolio(false);
        let mut warnings = vec![];
        let error = inventory_holding(&json!({"value":"not-an-integer","token":{"address_hash":"0x1111111111111111111111111111111111111111"}}), "0x3333333333333333333333333333333333333333", false, &mut 0, &lifi, &mut read, &mut warnings).unwrap_err();
        assert_eq!(error.code, "UPSTREAM_SCHEMA_CHANGED");
    }

    #[test]
    fn unknown_decimals_warn_even_without_quotes() {
        let (runtime, ctx) = fixture();
        let lifi = Lifi::from_ctx(&runtime, &ctx);
        let mut read = ReadContext::portfolio(false);
        let mut warnings = vec![];
        let holding = make_holding(
            "0x1111111111111111111111111111111111111111",
            None,
            None,
            None,
            "1",
            "0x3333333333333333333333333333333333333333",
            false,
            100,
            &mut 0,
            &lifi,
            &mut read,
            &mut warnings,
        )
        .unwrap();
        assert_eq!(holding["valuation"]["reason"], "unknown_decimals");
        assert_eq!(warnings[0]["code"], "UNKNOWN_DECIMALS");
    }

    #[test]
    fn quote_validation_checks_decimals_minimum_and_sums_gas() {
        let mut quote = json!({"action":{"fromToken":{"decimals":18},"toToken":{"decimals":6}},"estimate":{"toAmount":"100","toAmountMin":"90","gasCosts":[{"amountUSD":"0.1"},{"amountUSD":"0.2"}]}});
        assert!(quote_amounts(&quote, 18).is_some());
        assert_eq!(sum_gas_costs(&quote).as_deref(), Some("0.3"));
        quote["estimate"]["toAmountMin"] = json!("101");
        assert!(quote_amounts(&quote, 18).is_none());
    }

    #[test]
    fn holdings_sort_priced_first_then_by_token() {
        let mut holdings = vec![
            json!({"token":{"id":"b"},"valuation":{"value_usdg":null}}),
            json!({"token":{"id":"c"},"valuation":{"value_usdg":"2"}}),
            json!({"token":{"id":"a"},"valuation":{"value_usdg":"2"}}),
            json!({"token":{"id":"d"},"valuation":{"value_usdg":"3"}}),
        ];
        sort_holdings(&mut holdings);
        assert_eq!(
            holdings
                .iter()
                .map(|value| value["token"]["id"].as_str().unwrap())
                .collect::<Vec<_>>(),
            vec!["d", "a", "c", "b"]
        );
    }
}
