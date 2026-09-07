"use client";

import dynamic from "next/dynamic";
import Image from "next/image";
import Link from "next/link";
import { useSearchParams } from "next/navigation";
import { useRef, useState } from "react";
import "@aomi-labs/widget-lib/styles.css";
import "./app.css";

// The widget touches wallet APIs at import time, so it only loads in the browser.
const AomiWidget = dynamic(
  () => import("@aomi-labs/widget-lib").then((mod) => mod.AomiWidget),
  { ssr: false, loading: () => <div className="app-loading">Waking Hoodit…</div> },
);

const API_URL = process.env.NEXT_PUBLIC_AOMI_API_URL?.trim() || "https://chat.aomi.dev";
const CONFIGURED_APPLICATION_ID = process.env.NEXT_PUBLIC_AOMI_APPLICATION_ID?.trim();

const starters = [
  { label: "Explore stocks", text: "Find Apple Stock Tokens and show me the latest price.", icon: "↗" },
  { label: "Read the market", text: "What’s moving NVDA today? Show sources for the latest news.", icon: "◎" },
  { label: "Check my portfolio", text: "Show my Stock Token positions and their current reference value.", icon: "◫" },
  { label: "Plan a trade", text: "Help me review a $100 USDG-to-AAPL trade before I approve it.", icon: "⇄" },
];

function ChatPreview() {
  const [draft, setDraft] = useState("");
  const composer = useRef<HTMLTextAreaElement>(null);
  const choosePrompt = (text: string) => {
    setDraft(text);
    composer.current?.focus();
  };

  return (
    <div className="chat-preview">
      <aside className="chat-rail" aria-label="Chat workspace">
        <div className="rail-title">YOUR DESK</div>
        <button className="new-chat" onClick={() => choosePrompt("")}><span>＋</span> New chat</button>
        <div className="rail-history"><span>CONVERSATIONS</span><p>Your conversations will appear here once chat is available.</p></div>
        <div className="rail-note"><span>YOU KEEP THE FINAL SAY</span><p>Review the quote. Approve in your wallet. Stay in control.</p></div>
      </aside>
      <div className="chat-main">
        <header className="chat-header">
          <div className="chat-identity"><Image src="/hoodit-logo.jpg" alt="" width={40} height={40} /><div><strong>Hoodit</strong><span>Your Stock Token assistant</span></div></div>
          <span className="chat-status"><i /> Preview</span>
        </header>
        <div className="chat-welcome">
          <span className="chat-eyebrow">YOUR NEXT MOVE STARTS HERE</span>
          <h1>What’s the trade idea?</h1>
          <p>Explore Stock Tokens, understand the market, and turn an idea into a trade you can review.</p>
          <div className="chat-starters">
            {starters.map((starter) => <button key={starter.label} onClick={() => choosePrompt(starter.text)}><span className="starter-icon" aria-hidden="true">{starter.icon}</span><strong>{starter.label}</strong><span className="starter-text">{starter.text}</span><span className="starter-arrow" aria-hidden="true">↗</span></button>)}
          </div>
        </div>
        <div className="chat-compose-area">
          <p className="chat-availability" id="chat-availability"><span aria-hidden="true">◷</span> Chat is not available yet. You can draft a message here; nothing will be sent.</p>
          <form className="chat-composer" onSubmit={(event) => event.preventDefault()}>
            <label className="composer-label" htmlFor="hoodit-message">Message Hoodit</label>
            <textarea id="hoodit-message" ref={composer} value={draft} onChange={(event) => setDraft(event.target.value)} placeholder="Ask about a stock, your portfolio, or a trade…" rows={3} maxLength={4000} aria-describedby="chat-availability" />
            <div className="composer-toolbar"><span>Robinhood Stock Tokens</span><button type="submit" disabled aria-label="Send message — available when chat launches">↑</button></div>
          </form>
          <p className="chat-footnote">Stock Tokens are tokenized exposure, not underlying shares. Verify information before trading.</p>
        </div>
      </div>
    </div>
  );
}

export function HooditApp() {
  // `?application_id=` lets us point the page at a staging App without a rebuild.
  const params = useSearchParams();
  const applicationId = params.get("application_id")?.trim() || CONFIGURED_APPLICATION_ID;

  return (
    <main className="hoodit-app">
      <header className="app-bar">
        <Link className="brand" href="/" aria-label="Hoodit home">
          <Image className="brand-mark" src="/hoodit-logo.jpg" alt="" width={512} height={512} />
          <span>HOODIT</span>
        </Link>
        <span className="app-chain"><i /> ROBINHOOD CHAIN</span>
        <Link className="app-back" href="/">← Back to site</Link>
      </header>

      <section className="app-frame">
        {applicationId ? (
          <AomiWidget
            applicationId={applicationId}
            apiUrl={API_URL}
            auth={{ kind: "browser_wallet" }}
            wallets={{ evm: { preset: "popular" }, solana: false }}
            width="100%"
            height="calc(100dvh - 184px)"
            walletPosition="header"
            showSidebar
            showHeader
            persistThread
          />
        ) : (
          <ChatPreview />
        )}
      </section>

      <footer className="app-foot">
        <span>POWERED BY <b>AOMI</b></span>
        <span><i /> ROBINHOOD CHAIN</span>
      </footer>
    </main>
  );
}
