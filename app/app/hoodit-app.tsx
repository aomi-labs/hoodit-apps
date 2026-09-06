"use client";

import dynamic from "next/dynamic";
import Image from "next/image";
import Link from "next/link";
import { useSearchParams } from "next/navigation";
import "@aomi-labs/widget-lib/styles.css";
import "./app.css";

// The widget touches wallet APIs at import time, so it only loads in the browser.
const AomiWidget = dynamic(
  () => import("@aomi-labs/widget-lib").then((mod) => mod.AomiWidget),
  { ssr: false, loading: () => <div className="app-loading">Waking Hoodit…</div> },
);

const API_URL = process.env.NEXT_PUBLIC_AOMI_API_URL?.trim() || "https://chat.aomi.dev";
const CONFIGURED_APPLICATION_ID = process.env.NEXT_PUBLIC_AOMI_APPLICATION_ID?.trim();

export function HooditApp() {
  // `?application_id=` lets us point the page at a staging App without a rebuild.
  const params = useSearchParams();
  const applicationId = params.get("application_id")?.trim() || CONFIGURED_APPLICATION_ID;

  return (
    <main className="hoodit-app">
      <header className="app-bar">
        <Link className="brand" href="/" aria-label="Hoodit home">
          <Image className="brand-mark" src="/logo-cat.jpg" alt="" width={512} height={512} />
          <span>HOODIT</span>
        </Link>
        <span className="app-chain"><i /> LIVE ON ROBINHOOD CHAIN</span>
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
          <div className="app-setup">
            <p className="overline">ALMOST THERE</p>
            <h1>Hoodit is warming up.</h1>
            <p>
              The web app goes live once the Hoodit App is activated on Aomi. Until then, open
              Hoodit in Telegram or try the demo on the front page.
            </p>
            <div className="app-setup-actions">
              <Link className="primary-button" href="/#hero-qr">Open in Telegram <span>↗</span></Link>
              <Link className="text-link" href="/#demo">Try it first <span>↓</span></Link>
            </div>
            <small>
              Developers: set <code>NEXT_PUBLIC_AOMI_APPLICATION_ID</code> or append
              <code>?application_id=…</code> to this URL.
            </small>
          </div>
        )}
      </section>

      <footer className="app-foot">
        <span>POWERED BY <b>AOMI</b></span>
        <span><i /> ROBINHOOD CHAIN</span>
      </footer>
    </main>
  );
}
