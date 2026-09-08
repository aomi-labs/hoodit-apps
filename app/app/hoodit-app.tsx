"use client";

import dynamic from "next/dynamic";
import Link from "next/link";
import Image from "next/image";
import { chatFetch } from "../../lib/chat-fetch";
import "@aomi-labs/widget-lib/styles.css";
import "./app.css";

const AomiWidget = dynamic(
  () => import("@aomi-labs/widget-lib").then((mod) => mod.AomiWidget),
  { ssr: false, loading: () => <p>Loading Hoodit…</p> },
);

const clientOptions = { fetch: chatFetch };

export function HooditApp() {
  return (
    <main className="hoodit-app">
      <header className="app-bar">
        <Link className="brand" href="/" aria-label="Hoodit home">
          <Image className="brand-mark" src="/hoodit-logo.jpg" alt="" width={64} height={64} priority />
          <span>HOODIT</span>
        </Link>
        <span className="app-chain"><i /> ROBINHOOD CHAIN</span>
        <Link className="app-back" href="/">← Back to site</Link>
      </header>
      <section className="app-frame" aria-label="Hoodit chat">
        <AomiWidget
          applicationId="2938613"
          apiUrl="https://chat.aomi.dev"
          auth={{ kind: "browser_wallet" }}
          wallets={{ evm: { preset: "popular" }, solana: false }}
          walletFamilies={["evm"]}
          width="100%"
          height="100%"
          showHeader
          showSidebar
          walletPosition="header"
          clientOptions={clientOptions}
          controlBarProps={{ hideApp: true }}
          persistThread
          threadPersistenceKey="hoodit-chat"
        />
      </section>
      <footer className="app-foot">
        <span>POWERED BY <b>AOMI</b></span>
        <span>Stock Tokens are tokenized exposure, not underlying shares.</span>
      </footer>
    </main>
  );
}
