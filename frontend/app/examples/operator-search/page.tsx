"use client";

import { FormEvent, useEffect, useState } from "react";
import {
  getDatasetStatus,
  listOperatorStations,
  type DatasetStatus,
  type OperatorStationsResponse
} from "../../../src/lib/station-sdk";
import {
  DatasetBanner,
  ExamplePage,
  ResultSummary,
  SearchBand,
  StationList,
  StatusNotice
} from "../../../src/components/station-example-ui";

export default function OperatorSearchPage() {
  const [operatorName, setOperatorName] = useState("東日本旅客鉄道");
  const [result, setResult] = useState<OperatorStationsResponse | null>(null);
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

  async function runLookup(nextOperatorName: string) {
    setLoading(true);
    setError(null);

    try {
      setResult(await listOperatorStations(nextOperatorName));
    } catch (nextError) {
      setError(nextError instanceof Error ? nextError.message : "事業者検索に失敗しました。");
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
    await runLookup(operatorName);
  }

  useEffect(() => {
    void loadDataset();
  }, []);

  useEffect(() => {
    if (!datasetReady) {
      setResult(null);
      return;
    }
    void runLookup("東日本旅客鉄道");
  }, [datasetReady]);

  return (
    <ExamplePage
      title="事業者から駅一覧"
      description="広域の比較検討で、同じ事業者が持つ駅のまとまりをひと息で見渡すための画面です。"
      activeHref="/examples/operator-search"
      image={{
        src: "https://unsplash.com/photos/g16j5iIQ1Uc/download?force=true&w=1600",
        alt: "街の中に広がる駅周辺の風景"
      }}
    >
      <DatasetBanner dataset={dataset} loading={datasetLoading} />
      <SearchBand title="事業者から探す" detail="事業者ごとの駅を、路線順にまとめて見ます。">
        <form onSubmit={onSubmit} style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
          <input
            value={operatorName}
            onChange={(e) => setOperatorName(e.target.value)}
            disabled={!datasetReady}
            style={{
              flex: "1 1 320px",
              minHeight: 52,
              padding: "0 14px",
              border: "1px solid #b7c7a2",
              borderRadius: 8,
              background: datasetReady ? "#ffffff" : "#f3f4f6"
            }}
            placeholder="事業者名を入力"
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
      </SearchBand>

      <div style={{ marginTop: 18 }} />
      {!datasetReady && !datasetLoading ? (
        <StatusNotice tone="warning">全国駅データが揃うまで事業者一覧は停止します。</StatusNotice>
      ) : null}
      {loading ? <StatusNotice>事業者データを取得しています。</StatusNotice> : null}
      {error ? <StatusNotice tone="error">{error}</StatusNotice> : null}
      {result ? (
        <>
          <ResultSummary
            primary={`「${result.operator_name}」の駅一覧 ${result.items.length.toLocaleString()}件`}
            secondary="路線ごとのまとまりを崩さずに確認できます。"
          />
          <StationList
            items={result.items}
            emptyMessage={`「${result.operator_name}」に該当する駅は見つかりませんでした。`}
          />
        </>
      ) : null}
    </ExamplePage>
  );
}
