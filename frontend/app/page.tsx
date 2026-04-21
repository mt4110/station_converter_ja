"use client";

import { useDatasetOverview } from "../src/lib/use-dataset-overview";
import { DatasetBanner, DatasetHistoryPanels } from "../src/components/station-example-ui";

const pageInnerStyle = {
  width: "min(1080px, calc(100% - 32px))",
  margin: "0 auto"
} as const;

const cardStyle = {
  background: "#fff",
  border: "1px solid #d7dfc8",
  borderRadius: 8,
  padding: 20,
  display: "block",
  textDecoration: "none",
  color: "inherit"
} as const;

export default function HomePage() {
  const { dataset, datasetLoading, snapshots, changes, historyLoading, historyError } = useDatasetOverview({
    snapshotLimit: 3,
    changeLimit: 6
  });

  return (
    <main style={{ minHeight: "100vh", background: "#f5f7f0", color: "#1f2937" }}>
      <section style={{ background: "#67b600", color: "#ffffff", padding: "18px 0 24px" }}>
        <div style={pageInnerStyle}>
          <div style={{ fontSize: 12 }}>SAMPLE WEB</div>
          <h1 style={{ fontSize: 42, margin: "8px 0 0" }}>駅データサンプル</h1>
          <p style={{ fontSize: 18, lineHeight: 1.6, margin: "12px 0 0" }}>
            全国駅データを入れたあと、よく使う検索の入口と直近の変化をそのまま確かめられます。
          </p>
        </div>
      </section>

      <section style={{ padding: "18px 0 0" }}>
        <div style={pageInnerStyle}>
          <img
            src="https://unsplash.com/photos/1JGOthQNPq4/download?force=true&w=1600"
            alt="駅前広場と駅舎の風景"
            style={{
              display: "block",
              width: "100%",
              height: 220,
              objectFit: "cover",
              borderRadius: 8,
              border: "1px solid #d7dfc8"
            }}
          />
        </div>
      </section>

      <section style={{ padding: "18px 0 12px" }}>
        <div style={pageInnerStyle}>
          <DatasetBanner dataset={dataset} loading={datasetLoading} />
          <DatasetHistoryPanels
            dataset={dataset}
            snapshots={snapshots}
            changes={changes}
            loading={historyLoading}
            error={historyError}
          />
        </div>
      </section>

      <section style={{ padding: "6px 0 48px" }}>
        <div style={pageInnerStyle}>
          <div
            style={{
              fontSize: 13,
              color: "#43610f"
            }}
          >
            入口
          </div>
          <div style={{ fontSize: 24, fontWeight: 700, marginTop: 4 }}>
            よく使う順に入口を並べています。
          </div>

          <div style={{ display: "grid", gap: 16, marginTop: 18 }}>
            <a href="/examples/station-search" style={cardStyle}>
              <strong style={{ fontSize: 24 }}>駅名検索</strong>
              <div style={{ marginTop: 8 }}>同名駅を路線ごとに見分けます。</div>
            </a>

            <a href="/examples/line-search" style={cardStyle}>
              <strong style={{ fontSize: 24 }}>路線検索</strong>
              <div style={{ marginTop: 8 }}>路線名から停車駅一覧を引きます。</div>
            </a>

            <a href="/examples/operator-search" style={cardStyle}>
              <strong style={{ fontSize: 24 }}>事業者検索</strong>
              <div style={{ marginTop: 8 }}>事業者名から担当路線と駅のまとまりを確かめます。</div>
            </a>

            <a href="/examples/address-search" style={cardStyle}>
              <strong style={{ fontSize: 24 }}>住所から駅候補</strong>
              <div style={{ marginTop: 8 }}>住所や市区町村から近い駅候補と沿線確認まで続けます。</div>
            </a>

            <a href="/examples/nearby-search" style={cardStyle}>
              <strong style={{ fontSize: 24 }}>周辺検索</strong>
              <div style={{ marginTop: 8 }}>座標が分かっている案件向けに近い駅候補を返します。</div>
            </a>
          </div>
        </div>
      </section>
    </main>
  );
}
