import Image from "next/image";
import Link from "next/link";
import "./app.css";

const chatUrl = "https://chat.aomi.dev/?app=hoodit&application_id=2938613&lock_app=1";

export function HooditApp() {
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
      <section className="app-frame" aria-label="Hoodit chat">
        <iframe className="app-chat" src={chatUrl} title="Chat with Hoodit" />
      </section>
      <footer className="app-foot">
        <span>POWERED BY <b>AOMI</b></span>
        <a href={chatUrl} target="_blank" rel="noopener noreferrer">Open chat in a new tab ↗</a>
      </footer>
    </main>
  );
}
