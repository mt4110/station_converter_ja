"use client";

import { useEffect, useState } from "react";
import {
  listDatasetChanges,
  listDatasetSnapshots,
  type DatasetChangeEvent,
  type DatasetSnapshot
} from "./station-sdk";
import { loadDatasetStatus, type DatasetStatus } from "./dataset-status";

export function useDatasetOverview({
  snapshotLimit = 3,
  changeLimit = 6
}: {
  snapshotLimit?: number;
  changeLimit?: number;
} = {}) {
  const [dataset, setDataset] = useState<DatasetStatus | null>(null);
  const [snapshots, setSnapshots] = useState<DatasetSnapshot[]>([]);
  const [changes, setChanges] = useState<DatasetChangeEvent[]>([]);
  const [datasetLoading, setDatasetLoading] = useState(true);
  const [historyLoading, setHistoryLoading] = useState(true);
  const [historyError, setHistoryError] = useState<string | null>(null);

  useEffect(() => {
    async function loadOverview() {
      setDatasetLoading(true);
      setHistoryLoading(true);
      setHistoryError(null);

      try {
        const nextDataset = await loadDatasetStatus();
        setDataset(nextDataset);
        setDatasetLoading(false);

        if (!nextDataset.can_query_stations) {
          setSnapshots([]);
          setChanges([]);
          return;
        }

        try {
          const [nextSnapshots, nextChanges] = await Promise.all([
            listDatasetSnapshots(snapshotLimit),
            listDatasetChanges(changeLimit)
          ]);
          setSnapshots(nextSnapshots.items);
          setChanges(nextChanges.items);
        } catch (error) {
          const message = error instanceof Error ? error.message : "履歴の取得に失敗しました。";
          setHistoryError(message);
          setSnapshots([]);
          setChanges([]);
        }
      } catch {
        setDataset(null);
        setSnapshots([]);
        setChanges([]);
      } finally {
        setDatasetLoading(false);
        setHistoryLoading(false);
      }
    }

    void loadOverview();
  }, [changeLimit, snapshotLimit]);

  return {
    dataset,
    datasetLoading,
    datasetReady: dataset?.can_query_stations ?? false,
    snapshots,
    changes,
    historyLoading,
    historyError
  };
}
