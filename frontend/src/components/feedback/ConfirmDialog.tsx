import type { ReactNode } from "react";

type ConfirmDialogProps = {
  open: boolean;
  title: string;
  description: string;
  confirmLabel?: string;
  cancelLabel?: string;
  onConfirm: () => void;
  onCancel: () => void;
  children?: ReactNode;
};

export function ConfirmDialog({
  open,
  title,
  description,
  confirmLabel = "Confirm",
  cancelLabel = "Cancel",
  onConfirm,
  onCancel,
  children,
}: ConfirmDialogProps) {
  if (!open) {
    return null;
  }

  return (
    <div className="confirm-dialog" role="dialog" aria-modal="true" aria-label={title}>
      <div className="confirm-dialog__surface">
        <p className="section-header__eyebrow">confirmation</p>
        <h2>{title}</h2>
        <p>{description}</p>
        {children}
        <div className="action-bar__controls">
          <button className="button-link" onClick={onCancel} type="button">
            {cancelLabel}
          </button>
          <button className="button-link button-link--primary" onClick={onConfirm} type="button">
            {confirmLabel}
          </button>
        </div>
      </div>
    </div>
  );
}
