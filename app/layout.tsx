import type { Metadata } from "next";
import "@fontsource/archivo-black/400.css";
import "@fontsource/space-mono/400.css";
import "@fontsource/space-mono/700.css";
import "@fontsource/work-sans/400.css";
import "@fontsource/work-sans/500.css";
import "@fontsource/work-sans/600.css";
import "@fontsource/work-sans/700.css";
import "@fontsource/work-sans/800.css";
import "./globals.css";

const productionHost = process.env.VERCEL_PROJECT_PRODUCTION_URL;
const siteUrl = productionHost
  ? `https://${productionHost}`
  : "http://localhost:3000";

export const metadata: Metadata = {
  metadataBase: new URL(siteUrl),
  title: "Hoodit — Talk it. Trade it.",
  description:
    "Talk to an AI trading agent, review the transaction, and trade stocks on Robinhood Chain from Telegram or the web.",
  openGraph: {
    title: "Hoodit — Talk it. Trade it.",
    description: "AI trading on Robinhood Chain, in Telegram and on the web.",
    type: "website",
    images: [
      {
        url: `${siteUrl}/og.png`,
        width: 1672,
        height: 941,
        alt: "Hoodit — Talk it. Trade it.",
      },
    ],
  },
  twitter: {
    card: "summary_large_image",
    title: "Hoodit — Talk it. Trade it.",
    description: "AI trading on Robinhood Chain, in Telegram and on the web.",
    images: [`${siteUrl}/og.png`],
  },
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
