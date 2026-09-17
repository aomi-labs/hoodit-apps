use aomi_sdk::*;
mod amount;
pub mod app;
mod model;
mod providers;
pub mod tools;

const PREAMBLE: &str = include_str!("preamble.md");
const BLOCKSCOUT_API_KEY: Secret = Secret::new(
    "BLOCKSCOUT_API_KEY",
    "Free Blockscout API key for Robinhood Chain balance reads.",
    false,
);
const LIFI_API_KEY: Secret = Secret::new(
    "LIFI_API_KEY",
    "Optional LI.FI key for read-only valuation quotes.",
    false,
);

dyn_aomi_app!(
    app = app::HooditApp, name = "hoodit", version = "1.1.0", preamble = PREAMBLE,
    tools = [], secrets = [BLOCKSCOUT_API_KEY, LIFI_API_KEY], namespaces = ["aomi-core", "evm-core"],
    skills = [
        { id: "hoodit/markets", description: "Research Robinhood Chain tokens, pools, candles, and public trades", tags: ["markets", "tokens", "research"], tools: [tools::SearchTokens, tools::DiscoverPools, tools::GetToken, tools::GetCandles, tools::GetTrades], sections: { instructions: "skills/markets.md" }, },
        { id: "hoodit/portfolio", description: "Read exact public Robinhood Chain wallet balances and optional estimates", tags: ["wallet", "portfolio", "balances"], tools: [tools::GetPortfolio, tools::GetHolding], sections: { instructions: "skills/portfolio.md" }, },
    ],
);

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    #[test]
    fn manifest_has_only_skill_owned_v1_tools() {
        let manifest = app::HooditApp::default().manifest();
        assert_eq!(manifest.version, "1.1.0");
        assert_eq!(manifest.skills.len(), 2);
        assert_eq!(manifest.tools.len(), 7);
        let names = manifest
            .tools
            .iter()
            .map(|t| t.name.as_str())
            .collect::<HashSet<_>>();
        assert_eq!(names.len(), 7);
        assert!(
            !names
                .iter()
                .any(|n| n.contains("stock") || n.contains("probe"))
        );
        assert!(
            manifest
                .tools
                .iter()
                .all(|t| t.parameters_schema["additionalProperties"] == false)
        );
        assert!(
            manifest
                .secrets
                .as_ref()
                .unwrap()
                .iter()
                .all(|s| !s.required)
        );
    }
}
