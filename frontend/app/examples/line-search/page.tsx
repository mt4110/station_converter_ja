"use client";

import { FormEvent, useEffect, useMemo, useState } from "react";
import {
  getDatasetStatus,
  listLineCatalog,
  listLineStations,
  type LineCatalogEntry,
  type LineCatalogResponse,
  type DatasetStatus,
  type LineStationsResponse
} from "../../../src/lib/station-sdk";
import {
  DatasetBanner,
  ExamplePage,
  ResultSummary,
  SearchBand,
  StationList,
  StatusNotice
} from "../../../src/components/station-example-ui";

const lineModeButtonStyle = {
  minHeight: 40,
  padding: "0 14px",
  borderRadius: 8,
  fontWeight: 700,
  cursor: "pointer"
} as const;

const lineAreas = [
  { id: "kanto", label: "首都圏", lines: ["山手線", "京王線", "東横線", "江の島線"] },
  { id: "kansai", label: "関西", lines: ["大阪環状線", "京都線", "京阪本線", "南海本線"] },
  { id: "chubu", label: "中部", lines: ["名古屋本線", "犬山線", "三河線", "豊田線"] },
  { id: "tohoku", label: "東北", lines: ["仙石線", "仙山線", "常磐線", "磐越西線"] },
  { id: "hokkaido", label: "北海道", lines: ["千歳線", "南北線", "東西線", "東豊線"] },
  { id: "kyushu", label: "九州", lines: ["空港線", "筑肥線", "貝塚線", "日南線"] }
] as const;

type HelperMode = "line" | "area";

export default function LineSearchPage() {
  const [lineName, setLineName] = useState("山手線");
  const [result, setResult] = useState<LineStationsResponse | null>(null);
  const [dataset, setDataset] = useState<DatasetStatus | null>(null);
  const [loading, setLoading] = useState(false);
  const [datasetLoading, setDatasetLoading] = useState(true);
  const [catalog, setCatalog] = useState<LineCatalogResponse | null>(null);
  const [catalogLoading, setCatalogLoading] = useState(false);
  const [catalogError, setCatalogError] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [helperMode, setHelperMode] = useState<HelperMode>("line");
  const [selectedArea, setSelectedArea] = useState<(typeof lineAreas)[number]["id"]>("kanto");
  const datasetReady = dataset?.can_query_stations ?? false;
  const areaLines = lineAreas.find((area) => area.id === selectedArea)?.lines ?? [];
  const catalogEntries = catalog?.items ?? [];

  const filteredCatalogEntries = useMemo(() => {
    const keyword = lineName.trim();
    const baseEntries =
      helperMode === "area"
        ? catalogEntries.filter((entry) => areaLines.some((candidate) => candidate === entry.line_name))
        : keyword.length === 0
          ? catalogEntries
          : catalogEntries.filter(
              (entry) => entry.line_name.includes(keyword) || entry.operator_name.includes(keyword)
            );

    return {
      total: baseEntries.length,
      items: baseEntries.slice(0, 24)
    };
  }, [areaLines, catalogEntries, helperMode, lineName]);

  async function loadDataset() {
    setDatasetLoading(true);

    try {
      setDataset(await getDatasetStatus());
    } catch {
      setDataset(null);
    } finally {
      setDatasetLoading(false);
    }
  }

  async function loadCatalog() {
    setCatalogLoading(true);
    setCatalogError(null);

    try {
      setCatalog(await listLineCatalog("", 1000));
    } catch (nextError) {
      setCatalog(null);
      setCatalogError(nextError instanceof Error ? nextError.message : "路線候補の取得に失敗しました。");
    } finally {
      setCatalogLoading(false);
    }
  }

  async function runLookup(nextLineName: string) {
    setLoading(true);
    setError(null);

    try {
      setResult(await listLineStations(nextLineName));
    } catch (nextError) {
      setError(nextError instanceof Error ? nextError.message : "路線検索に失敗しました。");
      setResult(null);
    } finally {
      setLoading(false);
    }
  }

  async function onSubmit(event: FormEvent) {
    event.preventDefault();
    if (!datasetReady) {
      return;
    }
    await runLookup(lineName);
  }

  async function applyLineSuggestion(nextLineName: string) {
    setLineName(nextLineName);
    if (!datasetReady) {
      return;
    }
    await runLookup(nextLineName);
  }

  useEffect(() => {
    void loadDataset();
  }, []);

  useEffect(() => {
    if (!datasetReady) {
      setResult(null);
      setCatalog(null);
      return;
    }
    void loadCatalog();
    void runLookup("山手線");
  }, [datasetReady]);

  function catalogCaption(entry: LineCatalogEntry) {
    return `${entry.operator_name} / ${entry.station_count.toLocaleString()}駅`;
  }

  return (
    <ExamplePage
      title="路線から駅一覧"
      description="駅周辺比較の前に、路線ごとの停車駅をざっと把握するための一覧画面です。"
      activeHref="/examples/line-search"
      image={{
        src: "https://unsplash.com/photos/ZrO4EBTxfZo/download?force=true&w=1600",
        alt: "駅に停車する列車とホーム"
      }}
    >
      <DatasetBanner dataset={dataset} loading={datasetLoading} />
      <SearchBand title="路線から探す" detail="思い出せないときは、エリアから代表路線をたどれます。">
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap", marginBottom: 12 }}>
          <button
            type="button"
            onClick={() => setHelperMode("line")}
            style={{
              ...lineModeButtonStyle,
              border: helperMode === "line" ? "1px solid #4f8e00" : "1px solid #b7c7a2",
              background: helperMode === "line" ? "#67b600" : "#ffffff",
              color: helperMode === "line" ? "#ffffff" : "#2f6d00"
            }}
          >
            路線名
          </button>
          <button
            type="button"
            onClick={() => setHelperMode("area")}
            style={{
              ...lineModeButtonStyle,
              border: helperMode === "area" ? "1px solid #4f8e00" : "1px solid #b7c7a2",
              background: helperMode === "area" ? "#67b600" : "#ffffff",
              color: helperMode === "area" ? "#ffffff" : "#2f6d00"
            }}
          >
            エリア
          </button>
        </div>
        <form onSubmit={onSubmit} style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
          <input
            value={lineName}
            onChange={(e) => setLineName(e.target.value)}
            disabled={!datasetReady}
            style={{
              flex: "1 1 320px",
              minHeight: 52,
              padding: "0 14px",
              border: "1px solid #b7c7a2",
              borderRadius: 8,
              background: datasetReady ? "#ffffff" : "#f3f4f6"
            }}
            placeholder="路線名を入力"
          />
          <button
            type="submit"
            disabled={!datasetReady || loading}
            style={{
              minWidth: 140,
              minHeight: 52,
              padding: "0 18px",
              borderRadius: 8,
              border: "1px solid #4f8e00",
              background: !datasetReady || loading ? "#cbd5c0" : "#67b600",
              color: "#ffffff",
              fontWeight: 700
            }}
          >
            一覧表示
          </button>
        </form>
        <div style={{ marginTop: 12, display: "grid", gap: 10 }}>
          {helperMode === "area" ? (
            <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
              {lineAreas.map((area) => (
                <button
                  key={area.id}
                  type="button"
                  onClick={() => setSelectedArea(area.id)}
                  style={{
                    minHeight: 34,
                    padding: "0 10px",
                    borderRadius: 8,
                    border: area.id === selectedArea ? "1px solid #4f8e00" : "1px solid #cfe0bb",
                    background: area.id === selectedArea ? "#dff4bd" : "#ffffff",
                    color: "#2f6d00",
                    fontWeight: 700,
                    cursor: "pointer"
                  }}
                >
                  {area.label}
                </button>
              ))}
            </div>
          ) : null}
          <div style={{ fontSize: 13, color: "#4b5563" }}>
            {helperMode === "line"
              ? lineName.trim().length === 0
                ? "路線名を入れ始めると、候補を実データから絞り込みます。"
                : "候補を選ぶと、同名路線も事業者名つきで開けます。"
              : "エリアを先に絞って、その地域の路線候補から思い出せます。"}
          </div>
          {catalogLoading ? <div style={{ fontSize: 13, color: "#4b5563" }}>路線候補を読み込んでいます。</div> : null}
          {catalogError ? <StatusNotice tone="error">{catalogError}</StatusNotice> : null}
          {!catalogLoading && !catalogError ? (
            <>
              <div style={{ fontSize: 13, color: "#667085" }}>
                候補 {filteredCatalogEntries.total.toLocaleString()}件
                {filteredCatalogEntries.total > filteredCatalogEntries.items.length
                  ? ` / 先頭${filteredCatalogEntries.items.length.toLocaleString()}件を表示`
                  : ""}
              </div>
              <div
                style={{
                  display: "grid",
                  gridTemplateColumns: "repeat(auto-fit, minmax(180px, 1fr))",
                  gap: 8
                }}
              >
                {filteredCatalogEntries.items.map((entry) => {
                  const selected = entry.line_name === lineName;

                  return (
                    <button
                      key={`${entry.line_name}:${entry.operator_name}`}
                      type="button"
                      onClick={() => void applyLineSuggestion(entry.line_name)}
                      disabled={!datasetReady}
                      style={{
                        minHeight: 68,
                        padding: "10px 12px",
                        borderRadius: 8,
                        border: selected ? "1px solid #4f8e00" : "1px solid #cfe0bb",
                        background: selected ? "#dff4bd" : "#ffffff",
                        color: "#1f2937",
                        textAlign: "left",
                        cursor: "pointer"
                      }}
                    >
                      <div style={{ fontWeight: 700, color: "#2f6d00" }}>{entry.line_name}</div>
                      <div style={{ marginTop: 4, fontSize: 12, color: "#667085" }}>{catalogCaption(entry)}</div>
                    </button>
                  );
                })}
              </div>
              {!catalogLoading && filteredCatalogEntries.total === 0 ? (
                <div style={{ fontSize: 13, color: "#667085" }}>一致する路線候補はまだ見つかっていません。</div>
              ) : null}
            </>
          ) : null}
        </div>
      </SearchBand>

      <div style={{ marginTop: 18 }} />
      {!datasetReady && !datasetLoading ? (
        <StatusNotice tone="warning">全国駅データが揃うまで路線一覧は停止します。</StatusNotice>
      ) : null}
      {loading ? <StatusNotice>路線データを取得しています。</StatusNotice> : null}
      {error ? <StatusNotice tone="error">{error}</StatusNotice> : null}
      {result ? (
        <>
          <ResultSummary
            primary={`「${result.line_name}」の駅一覧 ${result.items.length.toLocaleString()}件`}
            secondary="同名路線の混在は事業者名を見て判断できます。"
          />
          <StationList
            items={result.items}
            emptyMessage={`「${result.line_name}」に該当する駅は見つかりませんでした。`}
          />
        </>
      ) : null}
    </ExamplePage>
  );
}
