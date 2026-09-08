"use client";

import dynamic from "next/dynamic";
import { useRef } from "react";
import Link from "next/link";
import Image from "next/image";
import "@aomi-labs/widget-lib/styles.css";
import { HooditBrand } from "./hoodit-brand";
import "./app.css";

const HooditWidget = dynamic(() => import("./hoodit-widget"), {
  ssr: false,
  loading: () => <p>Loading Hoodit…</p>,
});

export function HooditApp() {
  const frameRef = useRef<HTMLElement>(null);
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
      <section className="app-frame" aria-label="Hoodit chat" ref={frameRef}>
        <HooditWidget />
        <HooditBrand frameRef={frameRef} />
      </section>
      <footer className="app-foot">
        <span>© 2026 <b>HOODIT</b> · ROBINHOOD CHAIN</span>
        <span>Stock Tokens are tokenized exposure, not underlying shares.</span>
      </footer>
    </main>
  );
}
