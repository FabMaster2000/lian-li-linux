export type SectionStatus = "planned" | "next" | "ready";
export type SectionAccent = "copper" | "teal" | "ice";

export type SectionDescriptor = {
  id: string;
  title: string;
  description: string;
  status: SectionStatus;
  accent: SectionAccent;
  path: string;
};
