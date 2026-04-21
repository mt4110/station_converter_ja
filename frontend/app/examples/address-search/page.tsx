"use client";

import { FormEvent, useEffect, useMemo, useState } from "react";
import {
  listLineStations,
  searchNearbyStations,
  type LineStationsResponse,
  type NearbyStationsResponse,
  type StationSummary
} from "../../../src/lib/station-sdk";
import {
  searchAddressCandidates,
  type AddressCandidate,
  type AddressSearchResponse
} from "../../../src/lib/address-search";
import { useDatasetOverview } from "../../../src/lib/use-dataset-overview";
import {
  DatasetBanner,
  DatasetHistoryPanels,
  ExamplePage,
  ResultSummary,
  SearchBand,
  StatusNotice
} from "../../../src/components/station-example-ui";

const DEFAULT_QUERY = "東京都新宿区西新宿2-8-1";

const panelStyle = {
  border: "1px solid #d7dfc8",
  borderRadius: 8,
  background: "#ffffff",
  padding: 18
} as const;

function friendlyErrorMessage(error: unknown, fallback: string) {
  if (!(error instanceof Error)) {
    return fallback;
  }

  if (error.message.startsWith("API request failed")) {
    return fallback;
  }

  return error.message;
}

function formatDistanceMeters(distanceMeters: number) {
  if (distanceMeters < 1000) {
    return `${Math.round(distanceMeters)}m`;
  }

  return `${(distanceMeters / 1000).toFixed(1)}km`;
}

function distanceBetweenMeters(origin: AddressCandidate, station: StationSummary) {
  const earthRadiusMeters = 6_371_000;
  const originLat = (origin.latitude * Math.PI) / 180;
  const stationLat = (station.latitude * Math.PI) / 180;
  const deltaLat = ((station.latitude - origin.latitude) * Math.PI) / 180;
  const deltaLng = ((station.longitude - origin.longitude) * Math.PI) / 180;
  const a =
    Math.sin(deltaLat / 2) * Math.sin(deltaLat / 2) +
    Math.cos(originLat) * Math.cos(stationLat) * Math.sin(deltaLng / 2) * Math.sin(deltaLng / 2);
  const c = 2 * Math.atan2(Math.sqrt(a), Math.sqrt(1 - a));

  return earthRadiusMeters * c;
}

export default function AddressSearchPage() {
  const [query, setQuery] = useState(DEFAULT_QUERY);
  const [addressResult, setAddressResult] = useState<AddressSearchResponse | null>(null);
  const [selectedAddressIndex, setSelectedAddressIndex] = useState(0);
  const [nearbyResult, setNearbyResult] = useState<NearbyStationsResponse | null>(null);
  const [lineResult, setLineResult] = useState<LineStationsResponse | null>(null);
  const [selectedStationUid, setSelectedStationUid] = useState<string | null>(null);
  const [addressLoading, setAddressLoading] = useState(false);
  const [nearbyLoading, setNearbyLoading] = useState(false);
  const [lineLoading, setLineLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const { dataset, datasetLoading, datasetReady, snapshots, changes, historyLoading, historyError } =
    useDatasetOverview();
  const selectedAddress = addressResult?.items[selectedAddressIndex] ?? null;
  const selectedStation =
    nearbyResult?.items.find((item) => item.station_uid === selectedStationUid) ?? nearbyResult?.items[0] ?? null;

  const stationCandidates = useMemo(() => {
    if (!selectedAddress || !nearbyResult) {
      return [];
    }

    return nearbyResult.items.map((station) => ({
      station,
      distanceMeters: distanceBetweenMeters(selectedAddress, station)
    }));
  }, [nearbyResult, selectedAddress]);

  async function loadLineContext(station: StationSummary | null) {
    if (!station) {
      setLineResult(null);
      setSelectedStationUid(null);
      return;
    }

    setLineLoading(true);

    try {
      setLineResult(await listLineStations(station.line_name, station.operator_name));
      setSelectedStationUid(station.station_uid);
    } catch (nextError) {
      setLineResult(null);
      setError(friendlyErrorMessage(nextError, "沿線一覧の取得に失敗しました。"));
    } finally {
      setLineLoading(false);
    }
  }

  async function loadNearbyStations(location: AddressCandidate) {
    setNearbyLoading(true);

    try {
      const nextNearby = await searchNearbyStations(location.latitude, location.longitude, 8);
      setNearbyResult(nextNearby);
      await loadLineContext(nextNearby.items[0] ?? null);
    } catch (nextError) {
      setNearbyResult(null);
      setLineResult(null);
      setSelectedStationUid(null);
      setError(friendlyErrorMessage(nextError, "近い駅候補の取得に失敗しました。"));
    } finally {
      setNearbyLoading(false);
    }
  }

  async function runWorkflow(nextQuery: string) {
    setAddressLoading(true);
    setNearbyResult(null);
    setLineResult(null);
    setSelectedStationUid(null);
    setAddressResult(null);
    setSelectedAddressIndex(0);
    setError(null);

    try {
      const nextAddressResult = await searchAddressCandidates(nextQuery, 5);
      setAddressResult(nextAddressResult);

      if (nextAddressResult.items.length === 0) {
        return;
      }

      await loadNearbyStations(nextAddressResult.items[0]);
    } catch (nextError) {
      setError(friendlyErrorMessage(nextError, "住所検索サービスに接続できませんでした。"));
    } finally {
      setAddressLoading(false);
    }
  }

  async function onSubmit(event: FormEvent) {
    event.preventDefault();

    if (!datasetReady) {
      return;
    }

    await runWorkflow(query);
  }

  async function onSelectAddress(index: number) {
    if (!addressResult?.items[index]) {
      return;
    }

    setSelectedAddressIndex(index);
    setError(null);
    await loadNearbyStations(addressResult.items[index]);
  }

  async function onSelectStation(station: StationSummary) {
    setSelectedStationUid(station.station_uid);
    setError(null);

    if (
      lineResult?.line_name === station.line_name &&
      (lineResult.operator_name ?? null) === station.operator_name
    ) {
      return;
    }

    await loadLineContext(station);
  }

  useEffect(() => {
    if (!datasetReady) {
      setAddressResult(null);
      setNearbyResult(null);
      setLineResult(null);
      setSelectedStationUid(null);
      return;
    }

    void runWorkflow(DEFAULT_QUERY);
  }, [datasetReady]);

  return (
    <ExamplePage
      title="住所から駅候補"
      description="住所や市区町村から位置を解き、その地点に近い駅候補と沿線の見え方まで一画面で確認する導線です。"
      activeHref="/examples/address-search"
      image={{
        src: "https://unsplash.com/photos/peAzyTPKTlE/download?force=true&w=1600",
        alt: "駅と線路の見える街の風景"
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
      <SearchBand title="住所・市区町村から探す" detail="位置を解いて、近い駅候補と沿線の確認まで続けます。">
        <form onSubmit={onSubmit} style={{ display: "grid", gridTemplateColumns: "minmax(0, 1fr) auto", gap: 8 }}>
          <input
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            disabled={!datasetReady}
            style={{
              minHeight: 52,
              padding: "0 14px",
              border: "1px solid #b7c7a2",
              borderRadius: 8,
              background: datasetReady ? "#ffffff" : "#f3f4f6"
            }}
            placeholder="例: 東京都新宿区西新宿2-8-1 / 横浜市西区 / 札幌市中央区"
          />
          <button
            type="submit"
            disabled={!datasetReady || addressLoading || nearbyLoading}
            style={{
              minWidth: 156,
              minHeight: 52,
              padding: "0 18px",
              borderRadius: 8,
              border: "1px solid #4f8e00",
              background: !datasetReady || addressLoading || nearbyLoading ? "#cbd5c0" : "#67b600",
              color: "#ffffff",
              fontWeight: 700
            }}
          >
            駅候補を見る
          </button>
        </form>
        <div style={{ marginTop: 10, color: "#4b5563", lineHeight: 1.7 }}>
          住所はそのまま、市区町村だけでも検索できます。住所で引けないときは市区町村単位に寄せて再検索します。
        </div>
      </SearchBand>

      <div style={{ marginTop: 18 }} />
      {!datasetReady && !datasetLoading ? (
        <StatusNotice tone="warning">全国駅データが揃うまで住所からの候補出しは停止します。</StatusNotice>
      ) : null}
      {addressLoading ? <StatusNotice>住所と市区町村を位置に変換しています。</StatusNotice> : null}
      {nearbyLoading ? <StatusNotice>近い駅候補を集めています。</StatusNotice> : null}
      {lineLoading ? <StatusNotice>選んだ駅の沿線をまとめています。</StatusNotice> : null}
      {error ? <StatusNotice tone="error">{error}</StatusNotice> : null}

      {addressResult ? (
        <>
          <div style={{ marginTop: 18 }} />
          <ResultSummary
            primary={`「${addressResult.query}」から ${addressResult.items.length.toLocaleString()}件の位置候補`}
            secondary={
              addressResult.fallback_used
                ? `住所では引けず「${addressResult.resolved_query}」の代表点に寄せています。`
                : `位置解決は「${addressResult.resolved_query}」で行っています。`
            }
          />

          <section
            style={{
              display: "grid",
              gridTemplateColumns: "repeat(auto-fit, minmax(320px, 1fr))",
              gap: 16
            }}
          >
            <section style={panelStyle}>
              <div style={{ fontSize: 13, color: "#667085" }}>Step 1</div>
              <div style={{ fontSize: 22, fontWeight: 700, marginTop: 4 }}>位置候補</div>
              <div style={{ marginTop: 8, color: "#4b5563", lineHeight: 1.7 }}>
                複数ヒットしたときは、ここで地点を切り替えると周辺駅候補も更新されます。
              </div>
              {addressResult.items.length === 0 ? (
                <div style={{ marginTop: 16, color: "#b54708" }}>該当する住所・市区町村は見つかりませんでした。</div>
              ) : (
                <ul style={{ listStyle: "none", margin: "16px 0 0", padding: 0, display: "grid", gap: 10 }}>
                  {addressResult.items.map((item, index) => {
                    const active = selectedAddress?.title === item.title && selectedAddressIndex === index;

                    return (
                      <li key={`${item.title}-${item.latitude}-${item.longitude}`}>
                        <button
                          type="button"
                          onClick={() => void onSelectAddress(index)}
                          style={{
                            width: "100%",
                            textAlign: "left",
                            borderRadius: 8,
                            border: active ? "1px solid #67b600" : "1px solid #d7dfc8",
                            background: active ? "#f7fff0" : "#ffffff",
                            padding: 14,
                            cursor: "pointer"
                          }}
                        >
                          <div style={{ display: "flex", justifyContent: "space-between", gap: 12, flexWrap: "wrap" }}>
                            <strong style={{ fontSize: 18 }}>{item.title}</strong>
                            <span
                              style={{
                                display: "inline-flex",
                                alignItems: "center",
                                minHeight: 28,
                                padding: "0 10px",
                                borderRadius: 8,
                                background: active ? "#67b600" : "#eef8df",
                                color: active ? "#ffffff" : "#2f6d00",
                                fontSize: 13,
                                fontWeight: 700
                              }}
                            >
                              {active ? "表示中" : `候補 ${index + 1}`}
                            </span>
                          </div>
                          <div style={{ marginTop: 8, color: "#667085", fontSize: 14 }}>
                            {item.latitude.toFixed(6)}, {item.longitude.toFixed(6)}
                          </div>
                        </button>
                      </li>
                    );
                  })}
                </ul>
              )}
            </section>

            <section style={panelStyle}>
              <div style={{ fontSize: 13, color: "#667085" }}>Step 2</div>
              <div style={{ fontSize: 22, fontWeight: 700, marginTop: 4 }}>近い駅候補</div>
              <div style={{ marginTop: 8, color: "#4b5563", lineHeight: 1.7 }}>
                物件入力や問い合わせ票から、そのまま最寄り駅候補を絞る想定です。
              </div>
              {selectedAddress ? (
                <div
                  style={{
                    marginTop: 14,
                    padding: 12,
                    borderRadius: 8,
                    background: "#f7faf2",
                    border: "1px solid #d7dfc8"
                  }}
                >
                  <div style={{ fontSize: 12, color: "#667085" }}>基準地点</div>
                  <div style={{ marginTop: 4, fontWeight: 700 }}>{selectedAddress.title}</div>
                </div>
              ) : null}
              {stationCandidates.length === 0 ? (
                <div style={{ marginTop: 16, color: "#b54708" }}>近い駅候補はまだありません。</div>
              ) : (
                <ul style={{ listStyle: "none", margin: "16px 0 0", padding: 0, display: "grid", gap: 10 }}>
                  {stationCandidates.map(({ station, distanceMeters }, index) => {
                    const active = selectedStation?.station_uid === station.station_uid;

                    return (
                      <li key={station.station_uid}>
                        <button
                          type="button"
                          onClick={() => void onSelectStation(station)}
                          style={{
                            width: "100%",
                            textAlign: "left",
                            borderRadius: 8,
                            border: active ? "1px solid #67b600" : "1px solid #d7dfc8",
                            background: active ? "#f7fff0" : "#ffffff",
                            padding: 14,
                            cursor: "pointer"
                          }}
                        >
                          <div style={{ display: "flex", justifyContent: "space-between", gap: 12, flexWrap: "wrap" }}>
                            <div>
                              <div style={{ fontSize: 12, color: "#667085" }}>候補 {index + 1}</div>
                              <strong style={{ display: "block", fontSize: 20, marginTop: 4 }}>{station.station_name}</strong>
                            </div>
                            <span
                              style={{
                                display: "inline-flex",
                                alignItems: "center",
                                minHeight: 28,
                                padding: "0 10px",
                                borderRadius: 8,
                                background: active ? "#67b600" : "#eef8df",
                                color: active ? "#ffffff" : "#2f6d00",
                                fontSize: 13,
                                fontWeight: 700
                              }}
                            >
                              {formatDistanceMeters(distanceMeters)}
                            </span>
                          </div>
                          <div style={{ display: "flex", gap: 8, flexWrap: "wrap", marginTop: 10 }}>
                            <span
                              style={{
                                display: "inline-flex",
                                alignItems: "center",
                                minHeight: 28,
                                padding: "0 10px",
                                borderRadius: 8,
                                background: "#eef8df",
                                color: "#2f6d00",
                                fontWeight: 700,
                                fontSize: 13
                              }}
                            >
                              {station.line_name}
                            </span>
                            <span
                              style={{
                                display: "inline-flex",
                                alignItems: "center",
                                minHeight: 28,
                                padding: "0 10px",
                                borderRadius: 8,
                                border: "1px solid #d7dfc8",
                                background: "#ffffff",
                                fontSize: 13
                              }}
                            >
                              {station.operator_name}
                            </span>
                          </div>
                        </button>
                      </li>
                    );
                  })}
                </ul>
              )}
            </section>
          </section>

          <section style={{ ...panelStyle, marginTop: 16 }}>
            <div style={{ fontSize: 13, color: "#667085" }}>Step 3</div>
            <div style={{ fontSize: 22, fontWeight: 700, marginTop: 4 }}>沿線の見え方</div>
            <div style={{ marginTop: 8, color: "#4b5563", lineHeight: 1.7 }}>
              候補駅を選ぶと、その路線の停車駅をその場で確認できます。
            </div>

            {selectedStation ? (
              <div
                style={{
                  display: "flex",
                  justifyContent: "space-between",
                  gap: 12,
                  flexWrap: "wrap",
                  alignItems: "center",
                  marginTop: 16,
                  padding: 12,
                  borderRadius: 8,
                  background: "#f7faf2",
                  border: "1px solid #d7dfc8"
                }}
              >
                <div>
                  <div style={{ fontSize: 12, color: "#667085" }}>選択中の駅</div>
                  <strong style={{ display: "block", marginTop: 4, fontSize: 20 }}>{selectedStation.station_name}</strong>
                </div>
                <div style={{ color: "#4b5563" }}>{selectedStation.line_name}</div>
              </div>
            ) : null}

            {!lineResult ? (
              <div style={{ marginTop: 16, color: "#b54708" }}>沿線一覧はまだありません。</div>
            ) : (
              <>
                <div style={{ marginTop: 16, fontWeight: 700 }}>
                  「{lineResult.line_name}」の駅一覧 {lineResult.items.length.toLocaleString()}件
                </div>
                <ul
                  style={{
                    listStyle: "none",
                    margin: "14px 0 0",
                    padding: 0,
                    display: "grid",
                    gridTemplateColumns: "repeat(auto-fit, minmax(180px, 1fr))",
                    gap: 10
                  }}
                >
                  {lineResult.items.map((item) => {
                    const active = selectedStation?.station_uid === item.station_uid;

                    return (
                      <li
                        key={item.station_uid}
                        style={{
                          borderRadius: 8,
                          border: active ? "1px solid #67b600" : "1px solid #d7dfc8",
                          background: active ? "#f7fff0" : "#ffffff",
                          padding: 12
                        }}
                      >
                        <div style={{ fontWeight: 700 }}>{item.station_name}</div>
                        <div style={{ marginTop: 6, color: "#667085", fontSize: 13 }}>{item.operator_name}</div>
                      </li>
                    );
                  })}
                </ul>
              </>
            )}
          </section>
        </>
      ) : null}
    </ExamplePage>
  );
}
