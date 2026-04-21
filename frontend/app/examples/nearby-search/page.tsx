"use client";

import { FormEvent, useEffect, useState } from "react";
import {
  searchNearbyStations,
  type NearbyStationsResponse
} from "../../../src/lib/station-sdk";
import { useDatasetOverview } from "../../../src/lib/use-dataset-overview";
import {
  DatasetBanner,
  DatasetHistoryPanels,
  ExamplePage,
  ResultSummary,
  SearchBand,
  StationList,
  StatusNotice
} from "../../../src/components/station-example-ui";

export default function NearbySearchPage() {
  const [lat, setLat] = useState("35.6895");
  const [lng, setLng] = useState("139.6917");
  const [result, setResult] = useState<NearbyStationsResponse | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const { dataset, datasetLoading, datasetReady, snapshots, changes, historyLoading, historyError } =
    useDatasetOverview();

  async function runSearch(nextLat: string, nextLng: string) {
    setLoading(true);
    setError(null);

    try {
      setResult(await searchNearbyStations(Number(nextLat), Number(nextLng)));
    } catch (nextError) {
      setError(nextError instanceof Error ? nextError.message : "近傍検索に失敗しました。");
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
    await runSearch(lat, lng);
  }

  useEffect(() => {
    if (!datasetReady) {
      setResult(null);
      return;
    }
    void runSearch("35.6895", "139.6917");
  }, [datasetReady]);

  return (
    <ExamplePage
      title="近くの駅検索"
      description="住所や物件座標から、実際に近い駅候補を先に洗い出すための画面です。"
      activeHref="/examples/nearby-search"
      image={{
        src: "https://unsplash.com/photos/LAShlHKT390/download?force=true&w=1600",
        alt: "駅の案内サイン"
      }}
    >
      <DatasetBanner dataset={dataset} loading={datasetLoading} />
      <DatasetHistoryPanels
        dataset={dataset}
        snapshots={snapshots}
        changes={changes}
        loading={historyLoading}
        error={historyError}
      />
      <SearchBand title="座標から探す" detail="緯度経度から近い駅候補をまとめて返します。">
        <form
          onSubmit={onSubmit}
          style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))", gap: 8 }}
        >
          <input
            value={lat}
            onChange={(e) => setLat(e.target.value)}
            disabled={!datasetReady}
            style={{
              minHeight: 52,
              padding: "0 14px",
              border: "1px solid #b7c7a2",
              borderRadius: 8,
              background: datasetReady ? "#ffffff" : "#f3f4f6"
            }}
            placeholder="latitude"
          />
          <input
            value={lng}
            onChange={(e) => setLng(e.target.value)}
            disabled={!datasetReady}
            style={{
              minHeight: 52,
              padding: "0 14px",
              border: "1px solid #b7c7a2",
              borderRadius: 8,
              background: datasetReady ? "#ffffff" : "#f3f4f6"
            }}
            placeholder="longitude"
          />
          <button
            type="submit"
            disabled={!datasetReady || loading}
            style={{
              minHeight: 52,
              padding: "0 18px",
              borderRadius: 8,
              border: "1px solid #4f8e00",
              background: !datasetReady || loading ? "#cbd5c0" : "#67b600",
              color: "#ffffff",
              fontWeight: 700
            }}
          >
            周辺駅を表示
          </button>
        </form>
      </SearchBand>

      <div style={{ marginTop: 18 }} />
      {!datasetReady && !datasetLoading ? (
        <StatusNotice tone="warning">全国駅データが揃うまで周辺検索は停止します。</StatusNotice>
      ) : null}
      {loading ? <StatusNotice>近くの駅を検索しています。</StatusNotice> : null}
      {error ? <StatusNotice tone="error">{error}</StatusNotice> : null}
      {result ? (
        <>
          <ResultSummary
            primary={`(${result.query.lat.toFixed(4)}, ${result.query.lng.toFixed(4)}) 付近 ${result.items.length.toLocaleString()}件`}
            secondary="代表点ベースで近い順に返しています。"
          />
          <StationList
            items={result.items}
            emptyMessage="周辺駅は見つかりませんでした。"
          />
        </>
      ) : null}
    </ExamplePage>
  );
}
