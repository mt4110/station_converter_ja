"use client";

import { FormEvent, useState } from "react";
import { searchStations } from "../../../src/lib/station-sdk";

export default function StationSearchPage() {
  const [query, setQuery] = useState("新宿");
  const [result, setResult] = useState<any>(null);

  async function onSubmit(event: FormEvent) {
    event.preventDefault();
    const data = await searchStations(query);
    setResult(data);
  }

  return (
    <main style={{ maxWidth: 880, margin: "0 auto", padding: 32 }}>
      <h1>駅名検索</h1>
      <form onSubmit={onSubmit} style={{ display: "flex", gap: 8, marginBottom: 24 }}>
        <input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          style={{ flex: 1, padding: 12, border: "1px solid #d1d5db", borderRadius: 8 }}
          placeholder="駅名を入力"
        />
        <button type="submit" style={{ padding: "12px 18px" }}>検索</button>
      </form>

      <pre style={{ background: "#111827", color: "#f9fafb", padding: 16, borderRadius: 12, overflow: "auto" }}>
        {JSON.stringify(result, null, 2)}
      </pre>
    </main>
  );
}
