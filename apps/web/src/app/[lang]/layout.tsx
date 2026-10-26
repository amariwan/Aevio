import { Suspense } from "react";
import LangLayoutClient from "@/shell/LangLayoutClient";

export function generateStaticParams() {
  return [{ lang: "de" }, { lang: "en" }];
}

export default function LangLayout({ children }: { children: React.ReactNode }) {
  return (
    <Suspense fallback={null}>
      <LangLayoutClient>{children}</LangLayoutClient>
    </Suspense>
  );
}
