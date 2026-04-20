import type { ReactNode } from "react";

export const metadata = {
  title: "駅データサンプル",
  description: "全国駅データを使って検索導線を確認するサンプル"
};

export default function RootLayout({ children }: { children: ReactNode }) {
  return (
    <html lang="ja">
      <body style={{ margin: 0, fontFamily: "system-ui, sans-serif", background: "#fafafa", color: "#111" }}>
        {children}
      </body>
    </html>
  );
}
