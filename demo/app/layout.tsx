import type { Metadata } from "next";
import "./globals.css";
import { Providers } from "./providers";
import { Nav } from "@/components/Nav";

export const metadata: Metadata = {
  title: "Janus — Permissionless Binary Markets",
  description:
    "Onchain binary markets primitive on Solana. Mechanized at every layer.",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <body className="min-h-screen flex flex-col">
        <Providers>
          <Nav />
          <main className="flex-1">{children}</main>
          <Footer />
        </Providers>
      </body>
    </html>
  );
}

function Footer() {
  return (
    <footer className="border-t border-line mt-16">
      <div className="max-w-7xl mx-auto px-6 py-6 text-xs text-muted flex items-center justify-between">
        <div>© 2026 Janus. Permissionless binary markets on Solana.</div>
        <div className="flex gap-4">
          <a className="hover:text-black" href="https://github.com/saicharanpogul/janus" target="_blank" rel="noreferrer">GitHub</a>
          <a className="hover:text-black" href="https://explorer.solana.com/?cluster=devnet" target="_blank" rel="noreferrer">Explorer</a>
          <span>Network: devnet</span>
        </div>
      </div>
    </footer>
  );
}
