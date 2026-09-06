import type { Metadata } from "next";
import { HooditLanding } from "./hoodit-landing";

export const metadata: Metadata = {
  title: "Hoodit — Talk it. Trade it.",
  description:
    "The AI trading bot for stocks on Robinhood Chain, available in Telegram and on the web.",
};

export default function Home() {
  return <HooditLanding />;
}
