"use client";

import { FormEvent, useEffect, useState } from "react";
import {
  getDatasetStatus,
  searchStations,
  type DatasetStatus,
  type StationSearchResponse
} from "../../../src/lib/station-sdk";
import {
  DatasetBanner,
  ExamplePage,
  ResultSummary,
  SearchBand,
  StationList,
  StatusNotice
} from "../../../src/components/station-example-ui";

export default function StationSearchPage() {
  const [query, setQuery] = useState("新宿");
  const [result, setResult] = useState<StationSearchResponse | null>(null);
  const [dataset, setDataset] = useState<DatasetStatus | null>(null);
  const [loading, setLoading] = useState(false);
  const [datasetLoading, setDatasetLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const datasetReady = dataset?.can_query_stations ?? false;

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

  async function runSearch(nextQuery: string) {
    setLoading(true);
    setError(null);

    try {
      setResult(await searchStations(nextQuery));
    } catch (nextError) {
      setError(nextError instanceof Error ? nextError.message : "検索に失敗しました。");
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
    await runSearch(query);
  }

  useEffect(() => {
    void loadDataset();
  }, []);

  useEffect(() => {
    if (!datasetReady) {
      setResult(null);
      return;
    }
    void runSearch("新宿");
  }, [datasetReady]);

  return (
    <ExamplePage
      title="駅名検索"
      description="物件検索の最初の一手として、駅名から路線違いまで素早く見分けるための画面です。"
      activeHref="/examples/station-search"
      image={{
        src: "https://unsplash.com/photos/jIonE7aixKg/download?force=true&w=1600",
        alt: "駅ホームに停車する列車"
      }}
    >
      <DatasetBanner dataset={dataset} loading={datasetLoading} />
      <SearchBand title="駅名から探す" detail="同名駅の路線違いも一度に出します。">
        <form onSubmit={onSubmit} style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            disabled={!datasetReady}
            style={{
              flex: "1 1 300px",
              minHeight: 52,
              padding: "0 14px",
              border: "1px solid #b7c7a2",
              borderRadius: 8,
              background: datasetReady ? "#ffffff" : "#f3f4f6"
            }}
            placeholder="駅名を入力"
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
            検索
          </button>
        </form>
      </SearchBand>

      <div style={{ marginTop: 18 }} />
      {!datasetReady && !datasetLoading ? (
        <StatusNotice tone="warning">全国駅データが揃うまで検索フォームは停止します。</StatusNotice>
      ) : null}
      {loading ? <StatusNotice>検索しています。</StatusNotice> : null}
      {error ? <StatusNotice tone="error">{error}</StatusNotice> : null}
      {result ? (
        <>
          <ResultSummary
            primary={`「${result.query}」の検索結果 ${result.items.length.toLocaleString()}件`}
            secondary="駅名一致を優先して表示しています。"
          />
          <StationList
            items={result.items}
            emptyMessage={`「${result.query}」に一致する駅は見つかりませんでした。`}
          />
        </>
      ) : null}
    </ExamplePage>
  );
}
