const cardStyle = {
  background: "#fff",
  border: "1px solid #e5e7eb",
  borderRadius: 12,
  padding: 20,
  display: "block",
  textDecoration: "none",
  color: "inherit"
};

export default function HomePage() {
  return (
    <main style={{ maxWidth: 960, margin: "0 auto", padding: 32 }}>
      <h1 style={{ fontSize: 40, marginBottom: 8 }}>station_converter_ja</h1>
      <p style={{ fontSize: 18, lineHeight: 1.6 }}>
        Rust crawler + API と Next.js example frontend の雛形です。
      </p>

      <div style={{ display: "grid", gap: 16, marginTop: 24 }}>
        <a href="/examples/station-search" style={cardStyle}>
          <strong>駅名検索フォーム</strong>
          <div>駅名の検索 UI を試すための sample page</div>
        </a>

        <a href="/examples/line-search" style={cardStyle}>
          <strong>路線から駅一覧</strong>
          <div>路線単位で駅候補を返す sample page</div>
        </a>

        <a href="/examples/nearby-search" style={cardStyle}>
          <strong>近くの駅検索</strong>
          <div>緯度経度から近傍の駅を探す sample page</div>
        </a>
      </div>
    </main>
  );
}
