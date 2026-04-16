import type { ReactNode } from "react";

export const metadata = {
  title: "station_converter_ja examples",
  description: "Next.js example frontend for station_converter_ja"
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
