use crate::client::*;
use aomi_sdk::schemars::JsonSchema;
use aomi_sdk::*;
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::{Value, json};
use std::cmp::Ordering;
use std::str::FromStr;

pub(crate) struct SearchStockTokens;

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SearchStockTokensArgs {
    /// Ticker, company name, token name, or ISIN. An empty query lists active assets.
    query: String,
    /// Maximum results to return. Defaults to 10 and is capped at 50.
    #[serde(default)]
    limit: Option<usize>,
    /// Exclude inactive assets. Defaults to true.
    #[serde(default)]
    active_only: Option<bool>,
}

impl DynAomiTool for SearchStockTokens {
    type App = HooditApp;
    type Args = SearchStockTokensArgs;
    const NAME: &'static str = "hoodit_search_stock_tokens";
    const DESCRIPTION: &'static str = "Search Robinhood's live Stock Token catalog by ticker, company, token name, or ISIN. Use this before trading when the user's symbol is missing or ambiguous; returns canonical Robinhood Chain contracts but does not prepare a transaction.";

    fn run(_app: &HooditApp, args: Self::Args, _ctx: DynToolCallCtx) -> Result<Value, String> {
        let mut assets = HooditClient::new()?
            .assets()?
            .into_iter()
            .filter(|asset| !args.active_only.unwrap_or(true) || asset.is_active())
            .filter(|asset| asset.matches(&args.query))
            .filter(|asset| asset.robinhood_chain_contract().is_some())
            .collect::<Vec<_>>();

        assets.sort_by(|left, right| {
            asset_rank(left, &args.query).cmp(&asset_rank(right, &args.query))
        });
        let limit = args.limit.unwrap_or(10).clamp(1, 50);
        assets.truncate(limit);

        let assets = assets.iter().map(asset_json).collect::<Vec<_>>();
        Ok(json!({
            "source": "Robinhood Stock Token API /rhj/assets",
            "chain_id": ROBINHOOD_CHAIN_ID,
            "query": args.query,
            "count": assets.len(),
            "assets": assets,
        }))
    }
}

pub(crate) struct GetStockSnapshot;

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct GetStockSnapshotArgs {
    /// Exact Stock Token ticker, for example AAPL or NVDA. Search first when ambiguous.
    symbol: String,
}

impl DynAomiTool for GetStockSnapshot {
    type App = HooditApp;
    type Args = GetStockSnapshotArgs;
    const NAME: &'static str = "hoodit_get_stock_snapshot";
    const DESCRIPTION: &'static str = "Get trade-decision context for one Robinhood Stock Token: canonical contract, raw underlying bid/ask, corporate-action multiplier, adjusted token reference, halt status, data timestamp, and recent corporate actions. Call this before every buy or sell.";

    fn run(_app: &HooditApp, args: Self::Args, _ctx: DynToolCallCtx) -> Result<Value, String> {
        let symbol = normalize_symbol(&args.symbol)?;
        let client = HooditClient::new()?;
        let asset = client.asset(&symbol)?;
        let quote = client.quote(&symbol)?;
        let actions = client
            .corporate_actions()?
            .into_iter()
            .filter(|action| action.token_symbol.eq_ignore_ascii_case(&symbol))
            .take(5)
            .map(corporate_action_json)
            .collect::<Vec<_>>();

        let underlying_mid = decimal_midpoint(&quote.bid, &quote.ask);
        let token_bid = decimal_product(&[&quote.bid, &asset.current_multiplier]);
        let token_ask = decimal_product(&[&quote.ask, &asset.current_multiplier]);
        let token_mid = underlying_mid
            .as_deref()
            .and_then(|mid| decimal_product(&[mid, &asset.current_multiplier]));

        let mut warnings = Vec::new();
        if quote.is_trading_halt {
            warnings.push("The underlying equity is currently halted; do not prepare a trade.");
        }
        if !asset.is_active() {
            warnings.push("Robinhood marks this Stock Token inactive; do not prepare a trade.");
        }
        if !asset.pending_multiplier.is_empty() {
            warnings.push("A corporate-action multiplier change is pending; re-check immediately before trading.");
        }

        Ok(json!({
            "source": "Robinhood Stock Token API",
            "chain_id": ROBINHOOD_CHAIN_ID,
            "asset": asset_json(&asset),
            "underlying_market": quote_json(&quote),
            "token_reference": {
                "method": "raw underlying price multiplied by current_multiplier",
                "currency": quote.currency,
                "bid": token_bid,
                "ask": token_ask,
                "mid": token_mid,
                "current_multiplier": asset.current_multiplier,
                "not_an_executable_quote": true
            },
            "recent_corporate_actions": actions,
            "warnings": warnings,
            "execution_note": "Use the canonical resolver and a fresh LI.FI quote for an executable trade price."
        }))
    }
}

pub(crate) struct GetStockPosition;

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct GetStockPositionArgs {
    /// Exact Stock Token ticker, for example AAPL or NVDA.
    symbol: String,
    /// Optional EVM address. Omit to use the connected wallet.
    #[serde(default)]
    address: Option<String>,
}

impl DynAomiTool for GetStockPosition {
    type App = HooditApp;
    type Args = GetStockPositionArgs;
    const NAME: &'static str = "hoodit_get_stock_position";
    const DESCRIPTION: &'static str = "Read one canonical Robinhood Stock Token balance directly from Robinhood Chain and value it with Robinhood's current multiplier-adjusted reference. Use before selling, or when checking current exposure, without requiring an indexer.";

    fn run(_app: &HooditApp, args: Self::Args, ctx: DynToolCallCtx) -> Result<Value, String> {
        let symbol = normalize_symbol(&args.symbol)?;
        let address = args
            .address
            .or_else(|| ctx.attribute_string(&["domain", "evm", "address"]))
            .ok_or_else(|| {
                "Connect an EVM wallet or provide an address to view a position".to_string()
            })?;
        validate_evm_address(&address)?;

        let client = HooditClient::new()?;
        let asset = client.asset(&symbol)?;
        let contract = asset
            .robinhood_chain_contract()
            .ok_or_else(|| format!("{symbol} has no Robinhood Chain deployment"))?;
        let quote = client.quote(&symbol)?;
        let raw_balance = client.token_balance(contract, &address)?;
        let token_amount = format_units(&raw_balance, asset.token_decimals)
            .ok_or_else(|| "Could not format the onchain token balance".to_string())?;
        let underlying_mid = decimal_midpoint(&quote.bid, &quote.ask);
        let token_reference_mid = underlying_mid
            .as_deref()
            .and_then(|mid| decimal_product(&[mid, &asset.current_multiplier]));
        let reference_value_usd = token_reference_mid
            .as_deref()
            .and_then(|mid| decimal_product(&[&token_amount, mid]));

        Ok(json!({
            "sources": ["Robinhood Chain eth_call balanceOf", "Robinhood Stock Token API /rhj/assets and /rhj/prices"],
            "address": address,
            "chain_id": ROBINHOOD_CHAIN_ID,
            "connected_wallet_chain_id": ctx.attribute_u64(&["domain", "evm", "chain_id"]),
            "symbol": asset.token_symbol,
            "name": asset.token_name,
            "contract_address": contract,
            "raw_balance": raw_balance,
            "token_decimals": asset.token_decimals,
            "token_amount": token_amount,
            "current_multiplier": asset.current_multiplier,
            "underlying_shares_equivalent": decimal_product(&[&token_amount, &asset.current_multiplier]),
            "reference_mid_usd_per_token": token_reference_mid,
            "reference_value_usd": reference_value_usd,
            "price_generated_at": quote.generated_at,
            "is_trading_halt": quote.is_trading_halt,
            "valuation_note": "Reference value is not an executable quote and includes no cost basis or P/L."
        }))
    }
}

pub(crate) struct GetCorporateActions;

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct GetCorporateActionsArgs {
    /// Optional Stock Token ticker. Omit to list actions across all supported assets.
    #[serde(default)]
    symbol: Option<String>,
    /// When true, return only in-progress actions. Defaults to false.
    #[serde(default)]
    in_progress_only: Option<bool>,
    /// Maximum results. Defaults to 10 and is capped at 50.
    #[serde(default)]
    limit: Option<usize>,
}

impl DynAomiTool for GetCorporateActions {
    type App = HooditApp;
    type Args = GetCorporateActionsArgs;
    const NAME: &'static str = "hoodit_get_corporate_actions";
    const DESCRIPTION: &'static str = "List recent or pending Robinhood Stock Token corporate actions such as dividends and splits. Use before trading around an adjustment or when explaining a multiplier change.";

    fn run(_app: &HooditApp, args: Self::Args, _ctx: DynToolCallCtx) -> Result<Value, String> {
        let symbol = args.symbol.as_deref().map(normalize_symbol).transpose()?;
        let limit = args.limit.unwrap_or(10).clamp(1, 50);
        let actions = HooditClient::new()?
            .corporate_actions()?
            .into_iter()
            .filter(|action| {
                symbol
                    .as_deref()
                    .is_none_or(|symbol| action.token_symbol.eq_ignore_ascii_case(symbol))
            })
            .filter(|action| {
                !args.in_progress_only.unwrap_or(false)
                    || action.status == "CORPORATE_ACTION_STATUS_IN_PROGRESS"
            })
            .take(limit)
            .map(corporate_action_json)
            .collect::<Vec<_>>();

        Ok(json!({
            "source": "Robinhood Stock Token API /rhj/corporate-actions",
            "symbol": symbol,
            "count": actions.len(),
            "actions": actions,
        }))
    }
}

pub(crate) struct GetStockPortfolio;

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct GetStockPortfolioArgs {
    /// Optional EVM address. Omit to use the connected wallet.
    #[serde(default)]
    address: Option<String>,
    /// Maximum Stock Token positions to return. Defaults to 50 and is capped at 100.
    #[serde(default)]
    limit: Option<usize>,
}

impl DynAomiTool for GetStockPortfolio {
    type App = HooditApp;
    type Args = GetStockPortfolioArgs;
    const NAME: &'static str = "hoodit_get_stock_portfolio";
    const DESCRIPTION: &'static str = "Read canonical Robinhood Stock Token positions for an EVM address from Blockscout and value them with Robinhood's current multiplier-adjusted reference prices. Omit address to use the connected wallet. This is onchain wallet data, not a Robinhood brokerage portfolio.";

    fn run(_app: &HooditApp, args: Self::Args, ctx: DynToolCallCtx) -> Result<Value, String> {
        let address = args
            .address
            .or_else(|| ctx.attribute_string(&["domain", "evm", "address"]))
            .ok_or_else(|| {
                "Connect an EVM wallet or provide an address to view a portfolio".to_string()
            })?;
        validate_evm_address(&address)?;

        let client = HooditClient::new()?;
        let alchemy_api_key = ctx
            .secrets
            .get("ALCHEMY_API_KEY")
            .cloned()
            .or_else(|| std::env::var("ALCHEMY_API_KEY").ok());
        let blockscout_api_key = ctx
            .secrets
            .get("BLOCKSCOUT_API_KEY")
            .cloned()
            .or_else(|| std::env::var("BLOCKSCOUT_API_KEY").ok());
        let assets = client.assets()?;
        let quotes = client.quotes()?;
        let indexed = client.indexed_balances(
            &address,
            alchemy_api_key.as_deref(),
            blockscout_api_key.as_deref(),
        )?;
        let assets_by_contract = assets_by_contract(&assets);
        let quotes_by_symbol = quotes_by_symbol(&quotes);

        let mut positions = indexed
            .balances
            .into_iter()
            .filter_map(|balance| {
                let asset = assets_by_contract.get(&balance.contract_address.to_ascii_lowercase())?;
                let decimals = balance.decimals.unwrap_or(asset.token_decimals);
                let token_amount = format_units(&balance.raw_value, decimals)?;
                if token_amount == "0" {
                    return None;
                }
                let quote = quotes_by_symbol.get(&asset.token_symbol.to_ascii_uppercase());
                let underlying_mid = quote.and_then(|quote| decimal_midpoint(&quote.bid, &quote.ask));
                let token_reference_mid = underlying_mid
                    .as_deref()
                    .and_then(|mid| decimal_product(&[mid, &asset.current_multiplier]));
                let reference_value = token_reference_mid
                    .as_deref()
                    .and_then(|mid| decimal_product(&[&token_amount, mid]));
                Some(json!({
                    "symbol": asset.token_symbol,
                    "name": asset.token_name,
                    "contract_address": asset.robinhood_chain_contract(),
                    "token_amount": token_amount,
                    "raw_balance": balance.raw_value,
                    "token_decimals": decimals,
                    "current_multiplier": asset.current_multiplier,
                    "underlying_shares_equivalent": decimal_product(&[&token_amount, &asset.current_multiplier]),
                    "reference_mid_usd_per_token": token_reference_mid,
                    "reference_value_usd": reference_value,
                    "price_generated_at": quote.map(|quote| quote.generated_at.as_str()),
                    "is_trading_halt": quote.map(|quote| quote.is_trading_halt),
                }))
            })
            .collect::<Vec<_>>();

        positions.sort_by(|left, right| {
            let left = json_decimal(left.get("reference_value_usd"));
            let right = json_decimal(right.get("reference_value_usd"));
            right.partial_cmp(&left).unwrap_or(Ordering::Equal)
        });
        positions.truncate(args.limit.unwrap_or(50).clamp(1, 100));

        let reference_value_usd = positions
            .iter()
            .filter_map(|position| json_decimal(position.get("reference_value_usd")))
            .fold(Decimal::ZERO, |total, value| total + value)
            .normalize()
            .to_string();
        let connected_chain_id = ctx.attribute_u64(&["domain", "evm", "chain_id"]);

        Ok(json!({
            "sources": [format!("{} Robinhood Chain token balances", indexed.source), "Robinhood Stock Token API /rhj/assets and /rhj/prices"],
            "indexer_fallback_reason": indexed.fallback_reason,
            "address": address,
            "chain_id": ROBINHOOD_CHAIN_ID,
            "connected_wallet_chain_id": connected_chain_id,
            "position_count": positions.len(),
            "reference_value_usd": reference_value_usd,
            "positions": positions,
            "valuation_note": "Reference values use Robinhood's raw underlying midpoint multiplied by current_multiplier; they are not executable quotes and include no cost basis or P/L."
        }))
    }
}

fn asset_rank(asset: &Asset, query: &str) -> u8 {
    let query = query.trim().to_ascii_lowercase();
    if asset.token_symbol.eq_ignore_ascii_case(&query) {
        0
    } else if asset
        .token_name
        .to_ascii_lowercase()
        .strip_suffix(" • robinhood token")
        .is_some_and(|name| name == query)
    {
        1
    } else if asset.token_symbol.to_ascii_lowercase().starts_with(&query) {
        2
    } else {
        3
    }
}

fn asset_json(asset: &Asset) -> Value {
    let deployment = asset
        .deployments
        .iter()
        .find(|deployment| deployment.chain_id == ROBINHOOD_CHAIN_ID);
    json!({
        "id": asset.id,
        "symbol": asset.token_symbol,
        "name": asset.token_name,
        "isin": asset.isin,
        "status": asset.status,
        "chain_id": ROBINHOOD_CHAIN_ID,
        "contract_address": deployment.map(|deployment| deployment.contract_address.as_str()),
        "network_name": deployment.and_then(|deployment| deployment.network_name.as_deref()),
        "token_decimals": asset.token_decimals,
        "current_multiplier": asset.current_multiplier,
        "pending_multiplier": asset.pending_multiplier,
        "pending_multiplier_effective_time": asset.pending_multiplier_effective_time,
        "trading_capabilities": asset.trading_capabilities,
        "logo_url": asset.logo_url,
    })
}

fn quote_json(quote: &Quote) -> Value {
    json!({
        "symbol": quote.token_symbol,
        "bid": quote.bid,
        "ask": quote.ask,
        "mid": decimal_midpoint(&quote.bid, &quote.ask),
        "currency": quote.currency,
        "daily_high": quote.daily_high,
        "daily_low": quote.daily_low,
        "daily_trading_volume": quote.daily_trading_volume,
        "mint_burn_token_volume": quote.mint_burn_token_volume,
        "mint_burn_usd_volume": quote.mint_burn_usd_volume,
        "is_trading_halt": quote.is_trading_halt,
        "generated_at": quote.generated_at,
        "deployments": quote.deployments,
        "price_semantics": "raw underlying-equity price; not multiplier-adjusted"
    })
}

fn corporate_action_json(action: CorporateAction) -> Value {
    json!({
        "id": action.id,
        "symbol": action.token_symbol,
        "type": action.action_type,
        "status": action.status,
        "process_date": action.process_date,
        "details": action.details,
        "deployments": action.deployments,
    })
}

fn normalize_symbol(symbol: &str) -> Result<String, String> {
    let symbol = symbol.trim().to_ascii_uppercase();
    if symbol.is_empty()
        || symbol.len() > 12
        || !symbol
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-'))
    {
        return Err("Provide a valid Stock Token ticker such as AAPL or NVDA".to_string());
    }
    Ok(symbol)
}

fn validate_evm_address(address: &str) -> Result<(), String> {
    if address.len() == 42
        && address.starts_with("0x")
        && address[2..].bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        Ok(())
    } else {
        Err("Provide a valid 0x-prefixed EVM address".to_string())
    }
}

fn json_decimal(value: Option<&Value>) -> Option<Decimal> {
    value?
        .as_str()
        .and_then(|value| Decimal::from_str(value).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_symbols_and_addresses() {
        assert_eq!(normalize_symbol(" aapl ").as_deref(), Ok("AAPL"));
        assert!(normalize_symbol("AAPL/USD").is_err());
        assert!(validate_evm_address("0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045").is_ok());
        assert!(validate_evm_address("vitalik.eth").is_err());
    }
}
