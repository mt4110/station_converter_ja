"use client";

import { FormEvent, useState } from "react";
import { searchNearbyStations } from "../../../src/lib/station-sdk";

export default function NearbySearchPage() {
  const [lat, setLat] = useState("35.6895");
  const [lng, setLng] = useState("139.6917");
  const [result, setResult] = useState<any>(null);

  async function onSubmit(event: FormEvent) {
    event.preventDefault();
    const data = await searchNearbyStations(Number(lat), Number(lng));
    setResult(data);
  }

  return (
    <main style={{ maxWidth: 880, margin: "0 auto", padding: 32 }}>
      <h1>近くの駅検索</h1>
      <form onSubmit={onSubmit} style={{ display: "grid", gap: 8, marginBottom: 24 }}>
        <input
          value={lat}
          onChange={(e) => setLat(e.target.value)}
          style={{ padding: 12, border: "1px solid #d1d5db", borderRadius: 8 }}
          placeholder="latitude"
        />
        <input
          value={lng}
          onChange={(e) => setLng(e.target.value)}
          style={{ padding: 12, border: "1px solid #d1d5db", borderRadius: 8 }}
          placeholder="longitude"
        />
        <button type="submit" style={{ padding: "12px 18px", width: 180 }}>検索</button>
      </form>

      <pre style={{ background: "#111827", color: "#f9fafb", padding: 16, borderRadius: 12, overflow: "auto" }}>
        {JSON.stringify(result, null, 2)}
      </pre>
    </main>
  );
}
