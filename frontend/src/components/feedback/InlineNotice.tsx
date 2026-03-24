import type { ReactNode } from "react";

type InlineNoticeProps = {
  tone: "error" | "warning" | "success" | "info";
  title: string;
  children: ReactNode;
};

export function InlineNotice({ tone, title, children }: InlineNoticeProps) {
  return (
    <section className={`inline-notice inline-notice--${tone}`} role={tone === "error" ? "alert" : "status"}>
      <strong>{title}</strong>
      <span>{children}</span>
    </section>
  );
}
