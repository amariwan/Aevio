import type { Metadata } from "next";
import "@/styles/app.css";
import { Geist } from "next/font/google";
import { cn } from "@/lib/utils";

const geist = Geist({ subsets: ["latin"], variable: "--font-sans" });

export const metadata: Metadata = {
  title: "Aevio",
  description: "Time tracking, invoicing, and project management",
};

// Pre-paint appearance hint: Zinc and Rounded are the defaults, so paint
// them before styles run unless the last saved preferences (mirrored here
// by applyAppearance) say otherwise. Rust stays the source of truth -
// applyAppearance overwrites this as soon as settings load.
const appearanceHint = `(() => {
  const palettes = new Set([
    "zinc",
    "forest",
    "ocean",
    "violet",
    "sunset",
    "mono",
    "claude",
    "neobrutualism",
    "supabase",
    "vercel",
    "notebook",
    "light-green",
    "zen",
    "astro-vista",
    "whatsapp",
  ]);
  let palette = "zinc";
  try {
    const savedPalette = localStorage.getItem("aevio.theme");
    palette = savedPalette && palettes.has(savedPalette) ? savedPalette : "zinc";
  } catch {}
  document.documentElement.dataset.theme = palette;
  let mode = "system";
  try {
    mode = localStorage.getItem("aevio.mode") || "system";
  } catch {}
  document.documentElement.dataset.mode =
    mode === "system"
      ? window.matchMedia("(prefers-color-scheme: light)").matches
        ? "light"
        : "dark"
      : mode;
  let shape = "rounded";
  try {
    shape = localStorage.getItem("aevio.shape") || "rounded";
  } catch {}
  document.documentElement.dataset.shape = shape;
})();`;

export default function RootLayout({ children }: Readonly<{ children: React.ReactNode }>) {
  return (
    <html lang="en" className={cn("font-sans", geist.variable)} suppressHydrationWarning>
      <body>
        {children}
        <script dangerouslySetInnerHTML={{ __html: appearanceHint }} />
      </body>
    </html>
  );
}
