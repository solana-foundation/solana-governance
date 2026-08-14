"use client";
import { useEffect, useState } from "react";
import { GitHubIcon, TwitterIcon, DiscordIcon } from "./icons/SvgIcons";

type SocialLink = {
  href: string;
  ariaLabel: string;
  icon: React.ReactNode;
};

const socialLinks: SocialLink[] = [
  {
    href: "https://github.com/solana-foundation/solana-governance-proposals",
    ariaLabel: "Solana Governance Proposals on GitHub",
    icon: <GitHubIcon />,
  },
  {
    href: "https://x.com/solana",
    ariaLabel: "Solana on X",
    icon: <TwitterIcon />,
  },
  {
    href: "https://discord.gg/solana",
    ariaLabel: "Solana tech discord",
    icon: <DiscordIcon />,
  },
];

export default function Footer() {
  const [currentYear, setCurrentYear] = useState<number | null>(null);
  useEffect(() => setCurrentYear(new Date().getFullYear()), []);
  const renderSocialLinks = ({ href, ariaLabel, icon }: SocialLink) => (
    <a
      href={href}
      target="_blank"
      rel="noopener noreferrer"
      className="social-link"
      aria-label={ariaLabel}
      key={ariaLabel}
    >
      {icon}
    </a>
  );

  const renderLegalLinks = () => (
    <p className="legal-text">
      ©{currentYear !== null ? ` ${currentYear} ` : " "}Solana |{" "}
      <a
        href="https://solana.com/tos"
        target="_blank"
        rel="noopener noreferrer"
        className="legal-link"
      >
        Terms
      </a>{" "}
      |{" "}
      <a
        href="https://solana.com/privacy-policy"
        target="_blank"
        rel="noopener noreferrer"
        className="legal-link"
      >
        Privacy Policy
      </a>
    </p>
  );
  return (
    <footer className="footer-container">
      <div className="footer-social-container">
        {socialLinks.map((link) => renderSocialLinks(link))}
      </div>

      <div className="footer-legal-container">{renderLegalLinks()}</div>
    </footer>
  );
}
