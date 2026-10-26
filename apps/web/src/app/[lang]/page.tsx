"use client";

import { useEffect } from "react";
import { useRouter, useParams } from "next/navigation";

export default function Page() {
  const router = useRouter();
  const params = useParams();
  const lang = (params?.lang as string) ?? "en";
  useEffect(() => {
    router.replace(`/${lang}/dashboard`);
  }, [router, lang]);
  return null;
}
