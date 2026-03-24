type ErrorStateProps = {
  title: string;
  message: string;
};

export function ErrorState({ title, message }: ErrorStateProps) {
  return (
    <article className="error-state" role="alert">
      <h3>{title}</h3>
      <p>{message}</p>
    </article>
  );
}
