import type { Metadata } from "next";
import { Suspense } from "react";
import { HooditApp } from "./hoodit-app";

export const metadata: Metadata = {
  title: "Hoodit — App",
  description: "Talk to Hoodit and trade stocks on Robinhood Chain from the web.",
};

export default function AppPage() {
  return (
    <Suspense fallback={null}>
      <HooditApp />
    </Suspense>
  );
}
