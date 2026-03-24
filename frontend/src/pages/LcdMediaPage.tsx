import { useDocumentTitle } from "../hooks/useDocumentTitle";
import { FeaturePlaceholderPage } from "./FeaturePlaceholderPage";

export function LcdMediaPage() {
  useDocumentTitle("LCD / Media - Lian Li Control Surface");

  return (
    <FeaturePlaceholderPage
      description="Reserve the dedicated LCD and media workspace so the control suite can expose uploads, previews, and overlay workflows without overloading the existing device detail route."
      eyebrow="lcd / media"
      primaryDescription="This work area will later hold asset uploads, assignment workflows, previews, and overlay configuration where hardware support exists."
      primaryItems={[
        "Asset library and upload flow",
        "Current-assignment preview and validation state",
        "Overlay and sensor-widget controls where supported",
      ]}
      primaryTitle="Planned media work area"
      secondaryDescription="The route is available now to stabilize the target information architecture before the full feature phase lands."
      secondaryItems={[
        "Current LCD capability still appears through device capabilities",
        "Future backend and capability docs will define supported formats and conversion behavior",
        "Unsupported devices will receive explicit guidance instead of hidden actions",
      ]}
      secondaryTitle="Migration notes"
      title="LCD and media workspace"
    />
  );
}
