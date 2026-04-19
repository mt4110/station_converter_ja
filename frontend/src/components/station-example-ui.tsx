"use client";

import type { ReactNode } from "react";
import type { DatasetStatus, StationSummary } from "../lib/station-sdk";

const navItems = [
  { href: "/examples/station-search", label: "駅名検索" },
  { href: "/examples/line-search", label: "路線検索" },
  { href: "/examples/operator-search", label: "事業者検索" },
  { href: "/examples/address-search", label: "住所から駅候補" },
  { href: "/examples/nearby-search", label: "周辺検索" }
];

const pageInnerStyle = {
  width: "min(1080px, calc(100% - 32px))",
  margin: "0 auto"
} as const;

const panelStyle = {
  border: "1px solid #d7dfc8",
  borderRadius: 8,
  background: "#ffffff",
  padding: 16
} as const;

function googleMapsUrl(item: StationSummary) {
  return `https://www.google.com/maps/search/?api=1&query=${item.latitude},${item.longitude}`;
}

export function ExamplePage({
  title,
  description,
  activeHref,
  image,
  children
}: {
  title: string;
  description: string;
  activeHref: string;
  image: {
    src: string;
    alt: string;
  };
  children: ReactNode;
}) {
  return (
    <main style={{ minHeight: "100vh", background: "#f5f7f0", color: "#1f2937" }}>
      <section style={{ background: "#67b600", color: "#ffffff", padding: "18px 0 20px" }}>
        <div style={pageInnerStyle}>
          <div
            style={{
              display: "flex",
              justifyContent: "space-between",
              gap: 12,
              flexWrap: "wrap",
              alignItems: "center"
            }}
          >
            <div>
              <div style={{ fontSize: 12, letterSpacing: 0, opacity: 0.9 }}>SAMPLE WEB</div>
              <div style={{ fontSize: 22, fontWeight: 700, marginTop: 6 }}>駅データサンプル</div>
            </div>
            <div style={{ fontSize: 13, opacity: 0.94 }}>全国駅データを入れた状態で、そのまま確かめられます。</div>
          </div>
          <div style={{ marginTop: 18 }}>
            <h1 style={{ fontSize: 40, margin: 0 }}>{title}</h1>
            <p style={{ fontSize: 18, lineHeight: 1.6, margin: "10px 0 0" }}>{description}</p>
          </div>
          <nav style={{ display: "flex", gap: 10, flexWrap: "wrap", marginTop: 20 }}>
            {navItems.map((item) => {
              const active = item.href === activeHref;
              return (
                <a
                  key={item.href}
                  href={item.href}
                  style={{
                    display: "inline-flex",
                    alignItems: "center",
                    justifyContent: "center",
                    minHeight: 42,
                    padding: "0 16px",
                    borderRadius: 8,
                    textDecoration: "none",
                    border: active ? "1px solid #ffffff" : "1px solid rgba(255,255,255,0.45)",
                    background: active ? "#ffffff" : "rgba(255,255,255,0.12)",
                    color: active ? "#2f6d00" : "#ffffff",
                    fontWeight: 700
                  }}
                >
                  {item.label}
                </a>
              );
            })}
          </nav>
        </div>
      </section>

      <section style={{ padding: "18px 0 0" }}>
        <div style={pageInnerStyle}>
          <img
            src={image.src}
            alt={image.alt}
            style={{
              display: "block",
              width: "100%",
              height: 220,
              objectFit: "cover",
              borderRadius: 8,
              border: "1px solid #d7dfc8",
              background: "#dfe8cf"
            }}
          />
        </div>
      </section>

      <section style={{ padding: "18px 0 48px" }}>
        <div style={pageInnerStyle}>{children}</div>
      </section>
    </main>
  );
}

export function SearchBand({
  title,
  detail,
  children
}: {
  title: string;
  detail: string;
  children: ReactNode;
}) {
  return (
    <section
      style={{
        background: "#eef8df",
        borderTop: "4px solid #67b600",
        borderRadius: 8,
        padding: 18
      }}
    >
      <div style={{ fontSize: 13, color: "#43610f" }}>{title}</div>
      <div style={{ fontSize: 24, fontWeight: 700, marginTop: 4 }}>{detail}</div>
      <div style={{ marginTop: 16 }}>{children}</div>
    </section>
  );
}

export function DatasetBanner({
  dataset,
  loading
}: {
  dataset: DatasetStatus | null;
  loading: boolean;
}) {
  if (loading) {
    return (
      <section style={{ ...panelStyle, marginBottom: 16 }}>
        <strong>データ状態を確認中です。</strong>
      </section>
    );
  }

  if (!dataset) {
    return (
      <section style={{ ...panelStyle, marginBottom: 16, borderColor: "#d92d20", background: "#fff5f4" }}>
        <strong>API に接続できませんでした。</strong>
        <div style={{ marginTop: 8 }}>先に API を起動してから sample web を開いてください。</div>
      </section>
    );
  }

  const compatibilityMode = dataset.status_source === "legacy_search_probe";
  const ready = dataset.can_query_stations;
  const snapshotSourceLabel = (() => {
    const sourceVersion = dataset.active_snapshot?.source_version;
    const sourceUrl = dataset.active_snapshot?.source_url;

    if (dataset.source_is_local) {
      return sourceVersion ? `source: MLIT ${sourceVersion} / ローカル保存 ZIP` : "source: ローカル保存 ZIP";
    }

    if (sourceVersion) {
      return sourceUrl ? `source: MLIT ${sourceVersion} / ${sourceUrl}` : `source: MLIT ${sourceVersion}`;
    }

    return sourceUrl ? `source: ${sourceUrl}` : null;
  })();
  const stationCountLabel =
    dataset.active_station_count !== null && dataset.distinct_line_count !== null
      ? `${dataset.active_station_count.toLocaleString()}駅 / ${dataset.distinct_line_count.toLocaleString()}路線`
      : "件数は未確認";

  return (
    <section
      style={{
        ...panelStyle,
        marginBottom: 16,
        borderColor: compatibilityMode ? "#f1c36e" : ready ? "#b8d99b" : "#f1c36e",
        background: compatibilityMode ? "#fff8ec" : ready ? "#f7fff0" : "#fff8ec"
      }}
    >
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          gap: 12,
          flexWrap: "wrap",
          alignItems: "center"
        }}
      >
        <strong>
          {compatibilityMode
            ? "旧 API 互換モードで続行しています。"
            : ready
              ? "全国駅データを使える状態です。"
              : "全国駅データがまだ標準状態に達していません。"}
        </strong>
        <span
          style={{
            display: "inline-flex",
            alignItems: "center",
            minHeight: 30,
            padding: "0 10px",
            borderRadius: 8,
            border: "1px solid #d7dfc8",
            background: "#ffffff",
            fontSize: 13
          }}
        >
          {stationCountLabel}
        </span>
      </div>
      <div style={{ marginTop: 10, lineHeight: 1.7 }}>
        {compatibilityMode
          ? "データ状態の詳細は取れませんが、既存の駅検索応答を確認できたため画面を続行しています。"
          : ready
            ? "検索・一覧・近傍確認を通常モードで使えます。"
            : "Ingest N02 を完了するまで sample web の操作は止めます。"}
      </div>
      {ready && dataset.source_is_local ? (
        <div style={{ marginTop: 8, color: "#4b5563" }}>
          ローカルに保存した公式 ZIP から ingest したフルデータを表示しています。
        </div>
      ) : null}
      {compatibilityMode ? (
        <div style={{ marginTop: 8, color: "#667085", fontSize: 13 }}>
          件数は未確認でも検索は継続します。
        </div>
      ) : null}
      {snapshotSourceLabel ? (
        <div style={{ marginTop: 8, color: "#667085", wordBreak: "break-all", fontSize: 13 }}>
          {snapshotSourceLabel}
        </div>
      ) : null}
    </section>
  );
}

export function StatusNotice({
  children,
  tone = "neutral"
}: {
  children: ReactNode;
  tone?: "neutral" | "warning" | "error";
}) {
  const palette =
    tone === "error"
      ? { borderColor: "#d92d20", background: "#fff5f4" }
      : tone === "warning"
        ? { borderColor: "#f1c36e", background: "#fff8ec" }
        : { borderColor: "#d7dfc8", background: "#ffffff" };

  return <section style={{ ...panelStyle, ...palette }}>{children}</section>;
}

export function ResultSummary({
  primary,
  secondary
}: {
  primary: string;
  secondary: string;
}) {
  return (
    <section
      style={{
        display: "flex",
        justifyContent: "space-between",
        gap: 12,
        flexWrap: "wrap",
        alignItems: "center",
        marginBottom: 14
      }}
    >
      <div style={{ fontSize: 20, fontWeight: 700 }}>{primary}</div>
      <div style={{ color: "#667085" }}>{secondary}</div>
    </section>
  );
}

export function StationList({
  items,
  emptyMessage
}: {
  items: StationSummary[];
  emptyMessage: string;
}) {
  if (items.length === 0) {
    return <StatusNotice tone="warning">{emptyMessage}</StatusNotice>;
  }

  return (
    <ul style={{ listStyle: "none", padding: 0, margin: 0, display: "grid", gap: 12 }}>
      {items.map((item, index) => (
        <li
          key={item.station_uid}
          style={{
            display: "grid",
            gridTemplateColumns: "minmax(0, 1fr)",
            gap: 12,
            border: "1px solid #d7dfc8",
            borderRadius: 8,
            padding: 18,
            background: "#ffffff"
          }}
        >
          <div
            style={{
              display: "flex",
              justifyContent: "space-between",
              gap: 12,
              flexWrap: "wrap",
              alignItems: "flex-start"
            }}
          >
            <div>
              <div style={{ fontSize: 12, color: "#667085" }}>候補 {index + 1}</div>
              <strong style={{ display: "block", fontSize: 24, marginTop: 4 }}>{item.station_name}</strong>
            </div>
            <span
              style={{
                display: "inline-flex",
                alignItems: "center",
                minHeight: 30,
                padding: "0 10px",
                borderRadius: 8,
                border: "1px solid #d7dfc8",
                background: "#f7faf2",
                fontSize: 13,
                color: "#43610f"
              }}
            >
              {item.status}
            </span>
          </div>

          <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
            <span
              style={{
                display: "inline-flex",
                alignItems: "center",
                minHeight: 30,
                padding: "0 10px",
                borderRadius: 8,
                background: "#eef8df",
                color: "#2f6d00",
                fontWeight: 700
              }}
            >
              {item.line_name}
            </span>
            <span
              style={{
                display: "inline-flex",
                alignItems: "center",
                minHeight: 30,
                padding: "0 10px",
                borderRadius: 8,
                border: "1px solid #d7dfc8",
                background: "#ffffff"
              }}
            >
              {item.operator_name}
            </span>
          </div>

          <div
            style={{
              display: "grid",
              gridTemplateColumns: "repeat(auto-fit, minmax(180px, 1fr))",
              gap: 12,
              color: "#4b5563",
              fontSize: 14
            }}
          >
            <div>
              <div style={{ fontSize: 12, color: "#667085" }}>駅ID</div>
              <div style={{ marginTop: 4, wordBreak: "break-word" }}>{item.station_uid}</div>
            </div>
            <div>
              <div style={{ fontSize: 12, color: "#667085" }}>代表点</div>
              <div style={{ marginTop: 4, display: "flex", gap: 8, flexWrap: "wrap", alignItems: "center" }}>
                <span>
                  {item.latitude.toFixed(6)}, {item.longitude.toFixed(6)}
                </span>
                <a
                  href={googleMapsUrl(item)}
                  target="_blank"
                  rel="noreferrer"
                  style={{
                    display: "inline-flex",
                    alignItems: "center",
                    gap: 6,
                    minHeight: 30,
                    padding: "0 10px",
                    borderRadius: 8,
                    border: "1px solid #d7dfc8",
                    background: "#ffffff",
                    color: "#2f6d00",
                    textDecoration: "none",
                    fontSize: 13,
                    fontWeight: 700
                  }}
                  aria-label={`${item.station_name}の代表点を Google Maps で開く`}
                >
                  <span>Google Maps</span>
                  <svg
                    aria-hidden="true"
                    viewBox="0 0 16 16"
                    width="14"
                    height="14"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="1.5"
                  >
                    <path d="M6 3h7v7" />
                    <path d="M13 3 7 9" />
                    <path d="M10 8v4a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h4" />
                  </svg>
                </a>
              </div>
            </div>
          </div>
        </li>
      ))}
    </ul>
  );
}
