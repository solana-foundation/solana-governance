"use client";

import Link from "next/link";
import Image from "next/image";
import { usePathname } from "next/navigation";
import { AppButton } from "@/components/ui/AppButton";
import { Settings, Menu, LogOut } from "lucide-react";
import { useState, useEffect } from "react";
import {
  Drawer,
  DrawerClose,
  DrawerContent,
  DrawerDescription,
  DrawerHeader,
  DrawerTitle,
  DrawerTrigger,
} from "@/components/ui/drawer";
import { formatAddress } from "@/lib/governance/formatters";
import { useModal } from "@/contexts/ModalContext";
import { useWalletModal } from "@solana/wallet-adapter-react-ui";
import { useWallet } from "@solana/wallet-adapter-react";

type NavLink = {
  href: string;
  label: string;
  external?: boolean;
};

const navLinks: NavLink[] = [
  { href: "/proposals", label: "Proposals" },
  { href: "/dashboard", label: "Dashboard" },
  { href: "/faq", label: "FAQ" },
  {
    href: "https://docs.governance.solana.com",
    label: "Docs",
    external: true,
  },
];

const navLinkClassName = (isActive: boolean) =>
  `text-sm font-medium transition-colors ${
    isActive ? "text-white" : "text-muted hover:text-white"
  }`;

export default function Navbar() {
  const pathname = usePathname();
  const { openModal } = useModal();
  const [isOpen, setIsOpen] = useState(false);

  const { publicKey: walletAddress, connected, disconnect } = useWallet();

  const { setVisible } = useWalletModal();

  useEffect(() => {
    setIsOpen(false);
  }, [pathname]);

  const renderNavLinks = ({ href, label, external }: NavLink) => {
    const isActive = !external && pathname.startsWith(href);
    return (
      <li key={href}>
        {external ? (
          <a
            href={href}
            target="_blank"
            rel="noopener noreferrer"
            className={navLinkClassName(isActive)}
          >
            {label}
          </a>
        ) : (
          <Link href={href} className={navLinkClassName(isActive)}>
            {label}
          </Link>
        )}
      </li>
    );
  };
  return (
    <header className="sticky top-0 z-50 w-full backdrop-blur-md bg-transparent">
      <div className="mx-auto px-4 sm:px-8">
        <div className="flex h-20 items-center justify-between">
          <div className="flex items-center gap-4">
            <Drawer open={isOpen} onOpenChange={setIsOpen} direction="left">
              <DrawerTrigger asChild className="lg:hidden">
                <AppButton
                  type="button"
                  variant="outline"
                  size="icon"
                  aria-label="Menu"
                  icon={<Menu className="size-5" />}
                  className="text-muted rounded-full bg-transparent"
                />
              </DrawerTrigger>
              <DrawerContent
                className="h-full w-72 rounded-none border-r border-white/10 bg-background"
                tabIndex={0}
              >
                <DrawerHeader className="border-b border-white/10 px-4">
                  <DrawerTitle>
                    <Link href="/proposals" className="flex items-center gap-3">
                      <Image
                        src="/images/icons/solana.svg"
                        alt="Solana Logo"
                        width={0}
                        height={0}
                        className="shrink-0"
                        style={{ width: "24px", height: "auto" }}
                      />
                      <span className="text-lg font-bold text-foreground">
                        Validator Governance
                      </span>
                    </Link>
                  </DrawerTitle>
                  <DrawerDescription className="sr-only">
                    Navigation menu
                  </DrawerDescription>
                </DrawerHeader>

                <nav className="flex flex-col gap-2 p-4">
                  {navLinks.map((link) => {
                    const isActive = !link.external && pathname === link.href;
                    const className = `px-4 py-3 rounded-lg text-base font-medium transition-all ${
                      isActive
                        ? "bg-white/5 text-white"
                        : "text-muted hover:text-white hover:bg-white/5"
                    }`;
                    return (
                      <DrawerClose asChild key={link.href}>
                        {link.external ? (
                          <a
                            href={link.href}
                            target="_blank"
                            rel="noopener noreferrer"
                            className={className}
                          >
                            {link.label}
                          </a>
                        ) : (
                          <Link href={link.href} className={className}>
                            {link.label}
                          </Link>
                        )}
                      </DrawerClose>
                    );
                  })}

                  <div className="my-2 h-px bg-white/10" />

                  <button
                    type="button"
                    className="flex items-center gap-3 w-full px-4 py-3 rounded-lg text-base font-medium transition-all text-muted hover:text-white hover:bg-white/5"
                    aria-label="Settings"
                    onClick={() => {
                      openModal("settings");
                      setIsOpen(false);
                    }}
                  >
                    <Settings className="size-5" />
                    <span>Settings</span>
                  </button>
                </nav>
              </DrawerContent>
            </Drawer>

            {/* Desktop */}
            <Link href="/proposals" className="flex items-center gap-4">
              <Image
                src="/images/icons/solana.svg"
                alt="Solana Logo"
                width={0}
                height={0}
                className="shrink-0 hidden lg:block"
                style={{ width: "24px", height: "auto" }}
              />
              <span className="hidden lg:inline text-lg font-bold text-foreground tracking-tight">
                Solana Validator Governance
              </span>
            </Link>
          </div>
          <nav className="hidden lg:flex items-center gap-8 absolute left-1/2 -translate-x-1/2">
            <ul className="flex items-center gap-8 list-none">
              {navLinks.map((link) => renderNavLinks(link))}
            </ul>
          </nav>
          <div className="flex items-center gap-4">
            <AppButton
              type="button"
              variant="outline"
              size="icon"
              aria-label="Settings"
              icon={<Settings className="size-4" />}
              className="text-muted rounded-full hidden lg:flex"
              onClick={() => openModal("settings")}
            />
            {connected && walletAddress ? (
              <div className="flex items-center gap-4">
                <div className="flex items-center gap-4 flex-row-reverse">
                  <span className="size-2 rounded-full bg-emerald-400" />
                  <div className="flex flex-col items-end">
                    <span className="text-sm font-medium font-mono text-foreground">
                      {formatAddress(walletAddress.toString(), 4)}
                    </span>
                    <span className="text-xs text-emerald-400">Connected</span>
                  </div>
                </div>
                <AppButton
                  type="button"
                  variant="outline"
                  size="icon"
                  aria-label="Disconnect"
                  icon={<LogOut className="size-5" />}
                  className="text-muted border-none"
                  onClick={disconnect}
                />
              </div>
            ) : (
              <AppButton
                type="button"
                aria-label="Connect Wallet"
                variant="gradient"
                size="lg"
                className="rounded-full font-plus-jakarta-sans font-bold lg:h-9 lg:px-6"
                text="Connect Wallet"
                onClick={() => setVisible(true)}
              />
            )}
          </div>
        </div>
      </div>
    </header>
  );
}
