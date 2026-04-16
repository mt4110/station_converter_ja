"use client";

import { FormEvent, useState } from "react";

const baseUrl = process.env.NEXT_PUBLIC_STATION_API_BASE_URL ?? "http://localhost:3212";

export default function LineSearchPage() {
  const [lineName, setLineName] = useState("山手線");
  const [result, setResult] = useState<any>(null);

  async function onSubmit(event: FormEvent) {
    event.preventDefault();
    const response = await fetch(`${baseUrl}/v1/lines/${encodeURIComponent(lineName)}/stations`);
    const data = await response.json();
    setResult(data);
  }

  return (
    <main style={{ maxWidth: 880, margin: "0 auto", padding: 32 }}>
      <h1>路線から駅一覧</h1>
      <form onSubmit={onSubmit} style={{ display: "flex", gap: 8, marginBottom: 24 }}>
        <input
          value={lineName}
          onChange={(e) => setLineName(e.target.value)}
          style={{ flex: 1, padding: 12, border: "1px solid #d1d5db", borderRadius: 8 }}
          placeholder="路線名を入力"
        />
        <button type="submit" style={{ padding: "12px 18px" }}>取得</button>
      </form>

      <pre style={{ background: "#111827", color: "#f9fafb", padding: 16, borderRadius: 12, overflow: "auto" }}>
        {JSON.stringify(result, null, 2)}
      </pre>
    </main>
  );
}
