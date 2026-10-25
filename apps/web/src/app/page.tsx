"use client";

import dynamic from "next/dynamic";

// The window label decides which panel renders (main/pulse/reminder), and
// getCurrentWindow only exists in the browser — so the root must never
// prerender. Loading it client-only keeps the label dispatch flash-free.
const Root = dynamic(() => import("@/shell/Root"), { ssr: false });

export default function Page() {
  return (
    <div className="app-root">
      <Root />
    </div>
  );
}
