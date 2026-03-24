import type { ReactNode } from "react";
import { PageHeader } from "./layout/PageHeader";

type PageIntroProps = {
  eyebrow: string;
  title: string;
  description: string;
  aside?: ReactNode;
};

export function PageIntro({ eyebrow, title, description, aside }: PageIntroProps) {
  return <PageHeader actions={aside} description={description} eyebrow={eyebrow} title={title} />;
}
