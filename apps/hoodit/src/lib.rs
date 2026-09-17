use aomi_sdk::*;
mod amount;
pub mod app;
mod model;
mod providers;
pub mod tools;

const PREAMBLE: &str = include_str!("preamble.md");
dyn_aomi_app!(
    app = app::HooditApp, name = "hoodit", version = "1.1.3", preamble = PREAMBLE,
    tools = [], secrets = [], namespaces = ["aomi-core", "evm-core"],
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
        assert_eq!(manifest.version, "1.1.3");
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
        assert!(manifest.secrets.as_ref().is_none_or(Vec::is_empty));
        assert!(!manifest.preamble.contains("supply an API key"));
        assert!(
            manifest
                .preamble
                .contains("Never ask a user to supply a Blockscout or LI.FI API key")
        );
        assert!(manifest.preamble.contains(
            "Do not check or request provider credentials before calling Hoodit read tools"
        ));
        assert!(
            manifest
                .preamble
                .contains("For the first portfolio page, omit the `cursor` argument entirely")
        );
        assert!(
            manifest
                .preamble
                .contains("candle `before` values are Unix seconds")
        );
    }
}
