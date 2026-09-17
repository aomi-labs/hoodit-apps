use super::valuation::{
    HoldingInput, make_holding, normalize_inventory_holding, quote_token, sort_holdings,
    sum_holding_values, verify_usdg_token,
};
use crate::{
    app::{HooditApp, ReadContext},
    model,
    providers::{Blockscout, Lifi, ProviderError},
    tools::provider_error,
};
use aomi_sdk::schemars::JsonSchema;
use aomi_sdk::{DynAomiTool, DynToolCallCtx};
use serde::{Deserialize, Deserializer};
use serde_json::{Value, json};
use std::collections::HashSet;

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PortfolioArgs {
    /// Exact public 0x wallet address on Robinhood Chain. For "my wallet",
    /// resolve the funded executor with get_account_info on chain 4663 first.
    pub wallet_address: String,
    /// Opaque continuation returned by the previous portfolio response. Omit
    /// this field entirely for the first page: do not send JSON null, an empty
    /// string, or the string "null". For later pages, reuse only the exact
    /// next_cursor returned for this wallet; never invent a cursor.
    #[serde(
        default,
        deserialize_with = "first_page_cursor",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(
        with = "String",
        length(min = 1, max = 4096),
        pattern(r"^[A-Za-z0-9_-]+$")
    )]
    pub cursor: Option<String>,
    /// Estimate up to a bounded sample of holdings in USDG using LI.FI read
    /// quotes. Omit for false for a faster balance-only inventory read.
    #[serde(default)]
    #[schemars(with = "bool", extend("default" = false))]
    pub include_quotes: Option<bool>,
    /// Bypass Hoodit's short-lived read cache. Omit for false; use true only
    /// when the user explicitly asks for a fresh provider read.
    #[serde(default)]
    #[schemars(with = "bool", extend("default" = false))]
    pub refresh: Option<bool>,
}

pub struct GetPortfolio;

impl DynAomiTool for GetPortfolio {
    type App = HooditApp;
    type Args = PortfolioArgs;
    const NAME: &'static str = "hoodit_get_portfolio";
    const DESCRIPTION: &'static str = "Read one public Robinhood Chain wallet inventory page. Use for a portfolio or all-token balance request; omit cursor on the first page and only reuse this tool's next_cursor for the same wallet. Optional quotes are estimates, and the result contains no cost basis, P&L, or transaction history.";

    fn run(app: &HooditApp, args: PortfolioArgs, ctx: DynToolCallCtx) -> Result<Value, String> {
        let mut read = ReadContext::portfolio(args.refresh.unwrap_or(false));
        let wallet = match model::address(&args.wallet_address) {
            Ok(wallet) => wallet,
            Err(message) => return Ok(model::error("INVALID_ARGUMENT", &message, false)),
        };
        let runtime = app.runtime()?;
        let blockscout = match Blockscout::from_ctx(&runtime, &ctx) {
            Ok(blockscout) => blockscout,
            Err(error) => return Ok(provider_error(error)),
        };
        let inventory = match blockscout.inventory(&wallet, args.cursor.as_deref(), &mut read) {
            Ok(inventory) => inventory,
            Err(error) => return Ok(provider_error(error)),
        };
        let include_quotes = args.include_quotes.unwrap_or(false);
        if include_quotes && let Err(error) = verify_usdg_token(&blockscout, &mut read) {
            return Ok(provider_error(error));
        }
        let lifi = Lifi::from_ctx(&runtime, &ctx);
        let mut holdings = vec![];
        let mut warnings = vec![];
        let mut quote_attempts = 0;
        let mut seen_token_ids = HashSet::new();
        let native_balance = if args.cursor.is_none() {
            match blockscout.native_balance(&wallet, &mut read) {
                Ok(balance) => Some(balance),
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
        let native_included = native_balance.is_some();
        let mut returned = 0;
        for item in &inventory.items {
            match normalize_inventory_holding(
                item,
                &wallet,
                include_quotes,
                &mut quote_attempts,
                &lifi,
                &mut read,
                &mut warnings,
            ) {
                Ok(Some(holding)) => {
                    let token_id = model::string(&holding, &["token", "id"]).unwrap_or_default();
                    if !seen_token_ids.insert(token_id) {
                        return Ok(provider_error(ProviderError {
                            code: "UPSTREAM_SCHEMA_CHANGED",
                            message: "Blockscout returned a duplicate token holding".into(),
                            retryable: false,
                        }));
                    }
                    returned += 1;
                    holdings.push(holding);
                }
                Ok(None) => {}
                Err(error) => return Ok(provider_error(error)),
            }
        }
        if let Some(balance) = native_balance
            && balance != "0"
        {
            match make_holding(
                HoldingInput {
                    token_id: "native",
                    symbol: Some("ETH"),
                    name: Some("Ether"),
                    decimals: Some(18),
                    raw_balance: &balance,
                    wallet: &wallet,
                },
                include_quotes,
                100,
                &mut quote_attempts,
                &lifi,
                &mut read,
                &mut warnings,
            ) {
                Ok(holding) => holdings.push(holding),
                Err(error) => return Ok(provider_error(error)),
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
        let priced_count = holdings
            .iter()
            .filter(|holding| holding["valuation"]["value_usdg"].is_string())
            .count();
        let unpriced_count = holdings.len() - priced_count;
        let priced_value_usdg = sum_holding_values(&holdings);
        let total_value_usdg = if scope == "wallet" && include_quotes && unpriced_count == 0 {
            priced_value_usdg.clone().or_else(|| Some("0".into()))
        } else {
            None
        };
        Ok(model::ok(
            json!({"wallet_address":wallet,"quote_token":quote_token(include_quotes),"native_included":native_included,"holdings":holdings,"pagination":{"returned":returned,"next_cursor":inventory.next_cursor},"summary":{"scope":scope,"holdings_count":priced_count+unpriced_count,"priced_count":priced_count,"unpriced_count":unpriced_count,"priced_value_usdg":if include_quotes {priced_value_usdg}else{None},"total_value_usdg":total_value_usdg}}),
            read.sources,
            {
                warnings.extend(read.warnings);
                warnings
            },
        ))
    }
}

fn first_page_cursor<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let cursor = Option::<String>::deserialize(deserializer)?;
    Ok(cursor.and_then(|value| {
        let value = value.trim();
        if value.is_empty() || value.eq_ignore_ascii_case("null") {
            None
        } else {
            Some(value.to_string())
        }
    }))
}
