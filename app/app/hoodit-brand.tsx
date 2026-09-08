"use client";

/*
 * Hoodit branding layer for the embedded Aomi widget.
 *
 * `@aomi-labs/widget-lib` hard-codes its own mark, wordmark, composer
 * placeholder, and welcome suggestions. Visual swaps live in app.css; this
 * file handles the parts CSS cannot reach: placeholder text, the welcome
 * title, and a Hoodit-specific set of suggested actions that send through
 * the widget's own composer.
 */

import { useEffect, useState, type RefObject } from "react";
import { createPortal } from "react-dom";

const WELCOME_TITLE = "What are we trading on Robinhood Chain?";
const WELCOME_PLACEHOLDER = "Tell Hoodit what to trade…";
const REPLY_PLACEHOLDER = "Reply to Hoodit…";

const SUGGESTIONS = [
  { label: "Buy $100 of NVDA", prompt: "Buy $100 of NVDA stock token on Robinhood Chain" },
  { label: "What's TSLA trading at?", prompt: "What is TSLA stock token trading at right now on Robinhood Chain?" },
  { label: "Show my Robinhood Chain balances", prompt: "Show my wallet balances and stock token positions on Robinhood Chain" },
  { label: "Sell half my AAPL", prompt: "Sell half of my AAPL stock token position on Robinhood Chain" },
  { label: "Buy NVDA on breakout, 3% stop", prompt: "Buy $500 of NVDA if it breaks today's high, and keep downside under 3%" },
];

function patchText(root: HTMLElement) {
  root.querySelectorAll<HTMLTextAreaElement>("textarea.aui-composer-input").forEach((input) => {
    const want = input.closest(".aui-thread-welcome-root") ? WELCOME_PLACEHOLDER : REPLY_PLACEHOLDER;
    if (input.placeholder !== want) input.placeholder = want;
  });
  root.querySelectorAll<HTMLElement>(".aui-thread-welcome-title").forEach((title) => {
    if (title.textContent !== WELCOME_TITLE) title.textContent = WELCOME_TITLE;
  });
  root.querySelectorAll<HTMLButtonElement>('button[aria-label="Switch Aomi product"]').forEach((button) => {
    button.setAttribute("aria-label", "Hoodit");
    button.tabIndex = -1;
  });
}

/** Fill the widget's composer through React's own value setter, then send. */
function sendPrompt(root: HTMLElement, prompt: string) {
  const input = root.querySelector<HTMLTextAreaElement>("textarea.aui-composer-input");
  if (!input) return;
  const setValue = Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")?.set;
  setValue?.call(input, prompt);
  input.dispatchEvent(new Event("input", { bubbles: true }));
  requestAnimationFrame(() => {
    const send = root.querySelector<HTMLButtonElement>("button.aui-composer-send");
    if (send && !send.disabled) send.click();
    else input.focus();
  });
}

export function HooditBrand({ frameRef }: { frameRef: RefObject<HTMLElement | null> }) {
  const [suggestionHost, setSuggestionHost] = useState<HTMLElement | null>(null);

  useEffect(() => {
    const root = frameRef.current;
    if (!root) return;
    const sync = () => {
      patchText(root);
      const host = root.querySelector<HTMLElement>(".aui-thread-welcome-suggestions");
      setSuggestionHost((current) => (current === host ? current : host));
    };
    sync();
    const observer = new MutationObserver(sync);
    observer.observe(root, { childList: true, subtree: true, attributes: true, attributeFilter: ["placeholder"] });
    return () => observer.disconnect();
  }, [frameRef]);

  if (!suggestionHost) return null;
  return createPortal(
    <div className="hoodit-suggestions" role="group" aria-label="Suggested trades">
      {SUGGESTIONS.map((item) => (
        <button
          key={item.label}
          type="button"
          className="hoodit-suggestion"
          aria-label={item.prompt}
          onClick={() => frameRef.current && sendPrompt(frameRef.current, item.prompt)}
        >
          <i aria-hidden="true" />
          {item.label}
        </button>
      ))}
    </div>,
    suggestionHost,
  );
}
