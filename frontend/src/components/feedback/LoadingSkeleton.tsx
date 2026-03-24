type LoadingSkeletonProps = {
  title: string;
  message: string;
};

export function LoadingSkeleton({ title, message }: LoadingSkeletonProps) {
  return (
    <article aria-busy="true" className="loading-skeleton">
      <div className="loading-skeleton__line loading-skeleton__line--eyebrow" />
      <div className="loading-skeleton__line loading-skeleton__line--title" />
      <h3>{title}</h3>
      <p>{message}</p>
    </article>
  );
}
