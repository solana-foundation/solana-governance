import "./globals.css";
import type { Metadata } from "next";
import { Inter, Plus_Jakarta_Sans, JetBrains_Mono } from "next/font/google";
import { Suspense } from "react";
import Footer from "@/components/Footer";
import Navbar from "@/components/Navbar";
import { Toaster } from "@/components/ui/sonner";
import Providers from "./providers";
import { ModalProvider } from "@/contexts/ModalContext";
import { Analytics } from "@vercel/analytics/next";

const inter = Inter({
  variable: "--font-inter",
  subsets: ["latin"],
});

const plusJakartaSans = Plus_Jakarta_Sans({
  variable: "--font-plus-jakarta-sans",
  subsets: ["latin"],
});

const jetBrainsMono = JetBrains_Mono({
  variable: "--font-jetbrains-mono",
  subsets: ["latin"],
});

export const metadata: Metadata = {
  title: "Solana Validator Governance",
  description: "Vote and participate in Solana validator governance",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" suppressHydrationWarning>
      <body
        className={`${inter.variable} ${plusJakartaSans.variable} ${jetBrainsMono.variable} antialiased`}
      >
        <Providers>
          <ModalProvider>
            <Suspense fallback={null}>
              <Navbar />
              <div className="w-full overflow-x-hidden">
                <div className="max-w-7xl mx-auto px-4 sm:px-8">{children}</div>
              </div>
              <Footer />
            </Suspense>
            <Toaster theme="dark" position="bottom-right" />
          </ModalProvider>
        </Providers>
        <Analytics />
      </body>
    </html>
  );
}
