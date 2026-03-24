type EmptyStateProps = {
  title: string;
  message: string;
};

export function EmptyState({ title, message }: EmptyStateProps) {
  return (
    <article className="empty-state">
      <h3>{title}</h3>
      <p>{message}</p>
    </article>
  );
}
