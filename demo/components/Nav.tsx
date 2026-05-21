"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import dynamic from "next/dynamic";
import clsx from "clsx";

const WalletMultiButton = dynamic(
  async () =>
    (await import("@solana/wallet-adapter-react-ui")).WalletMultiButton,
  { ssr: false },
);

const sections = [
  { href: "/", label: "Portfolio" },
  { href: "/markets", label: "Markets" },
  { href: "/create", label: "Create" },
  { href: "/swarm", label: "Swarm" },
];

export function Nav() {
  const pathname = usePathname();
  return (
    <header className="border-b border-line">
      {/* Bloomberg-style top utility bar */}
      <div className="bg-black text-white text-xs">
        <div className="max-w-7xl mx-auto px-6 h-8 flex items-center gap-6">
          <span className="font-semibold">Janus Markets</span>
          <span className="text-muted">|</span>
          <a className="hover:text-accent" href="https://github.com/saicharanpogul/janus" target="_blank" rel="noreferrer">
            Docs
          </a>
          <a className="hover:text-accent" href="https://github.com/saicharanpogul/janus/blob/main/DEVNET.md" target="_blank" rel="noreferrer">
            Devnet Status
          </a>
          <span className="ml-auto text-accent font-medium">DEVNET</span>
        </div>
      </div>
      {/* Brand + section nav + wallet */}
      <div className="max-w-7xl mx-auto px-6 h-16 flex items-center justify-between">
        <Link href="/" className="flex items-center gap-3">
          <span className="text-2xl font-extrabold tracking-tight">Janus</span>
        </Link>
        <nav className="hidden md:flex items-center gap-8">
          {sections.map((s) => (
            <Link
              key={s.href}
              href={s.href}
              className={clsx(
                "text-sm font-medium hover:text-black",
                pathname === s.href
                  ? "text-black border-b-2 border-black h-16 flex items-center"
                  : "text-muted",
              )}
            >
              {s.label}
            </Link>
          ))}
        </nav>
        <div className="flex items-center">
          <WalletMultiButton />
        </div>
      </div>
    </header>
  );
}
