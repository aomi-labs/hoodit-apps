import type { Metadata } from "next";
import { redirect } from "next/navigation";

export const metadata: Metadata = {
  title: "Hoodit — App",
  description: "Talk to Hoodit and trade stocks on Robinhood Chain from the web.",
};

export default function AppPage() {
  // Cross-site embeds cannot reliably establish the hosted chat session.
  // Keep authentication first-party until the embeddable client supports it.
  redirect("https://chat.aomi.dev/?app=hoodit&application_id=2938613&lock_app=1");
}
