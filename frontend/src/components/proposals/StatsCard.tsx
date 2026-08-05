import { formatNumber } from "@/helpers";

type StatsCardProps = {
  label: string;
  /** `undefined` when the value could not be loaded; renders "-". */
  value: number | undefined;
  isLoading: boolean;
};

export default function StatsCard({ label, value, isLoading }: StatsCardProps) {
  return (
    <article className="glass-card px-2 py-4 sm:p-6 flex h-full flex-col justify-between gap-4">
      <div className="proposal-stats-card-content">
        <div className="proposal-stats-card-wrapper">
          <p className="proposal-stats-card-label">{label}</p>
          <div className="proposal-stats-card-value flex justify-center">
            {isLoading ? (
              <span className="animate-pulse h-8 w-10 bg-white/10 rounded" />
            ) : (
              (value === undefined ? "-" : formatNumber(value))
            )}
          </div>
        </div>
      </div>
    </article>
  );
}
