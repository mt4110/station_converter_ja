const cardStyle = {
  background: "#fff",
  border: "1px solid #d7dfc8",
  borderRadius: 8,
  padding: 20,
  display: "block",
  textDecoration: "none",
  color: "inherit"
};

export default function HomePage() {
  return (
    <main style={{ minHeight: "100vh", background: "#f5f7f0", color: "#1f2937" }}>
      <section style={{ background: "#67b600", color: "#ffffff", padding: "18px 0 24px" }}>
        <div style={{ width: "min(1080px, calc(100% - 32px))", margin: "0 auto" }}>
          <div style={{ fontSize: 12 }}>SAMPLE WEB</div>
          <h1 style={{ fontSize: 42, margin: "8px 0 0" }}>駅データサンプル</h1>
          <p style={{ fontSize: 18, lineHeight: 1.6, margin: "12px 0 0" }}>
            全国駅データを入れたあと、よく使う検索の入口をそのまま試せます。
          </p>
        </div>
      </section>

      <section style={{ padding: "18px 0 0" }}>
        <div style={{ width: "min(1080px, calc(100% - 32px))", margin: "0 auto" }}>
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

      <section style={{ width: "min(1080px, calc(100% - 32px))", margin: "0 auto", padding: "18px 0 48px" }}>
        <div
          style={{
            background: "#eef8df",
            borderTop: "4px solid #67b600",
            borderRadius: 8,
            padding: 18
          }}
        >
          <div style={{ fontSize: 13, color: "#43610f" }}>入口</div>
          <div style={{ fontSize: 24, fontWeight: 700, marginTop: 4 }}>よく使う順に入口を並べています。</div>
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
      </section>
    </main>
  );
}
