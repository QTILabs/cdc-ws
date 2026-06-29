import { useQuery } from '@tanstack/react-query';
import { cdcApi } from '../api/cdc';
import MetricsCard from '../components/MetricsCard';
import { Activity, CheckCircle, XCircle, AlertTriangle } from 'lucide-react';

export default function Dashboard() {
  const { data: health } = useQuery({
    queryKey: ['health'],
    queryFn: cdcApi.getHealth,
    refetchInterval: 5000,
  });

  const { data: metrics } = useQuery({
    queryKey: ['metrics'],
    queryFn: cdcApi.getMetrics,
    refetchInterval: 3000,
  });

  return (
    <div>
      <div className="mb-8">
        <h1 className="text-2xl font-bold text-slate-900">Dashboard</h1>
        <p className="text-sm text-slate-500 mt-1">Real-time CDC daemon monitoring</p>
      </div>

      {/* Health status banner */}
      {health && (
        <div
          className={`mb-6 p-4 rounded-xl border ${
            health.is_healthy
              ? 'bg-green-50 border-green-200 text-green-800'
              : 'bg-red-50 border-red-200 text-red-800'
          }`}
        >
          <div className="flex items-center gap-3">
            {health.is_healthy ? <CheckCircle size={20} /> : <AlertTriangle size={20} />}
            <div>
              <p className="font-semibold">
                System Status: {health.overall_status}
              </p>
              <p className="text-sm opacity-80">
                {Object.entries(health.components)
                  .map(([k, v]) => `${k}: ${v}`)
                  .join(' • ')}
              </p>
            </div>
          </div>
        </div>
      )}

      {/* Metrics grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <MetricsCard
          title="Records Ingested"
          value={metrics?.records_ingested ?? 0}
          icon={<Activity size={24} />}
          color="blue"
        />
        <MetricsCard
          title="Successfully Sunk"
          value={metrics?.records_sunk_success ?? 0}
          icon={<CheckCircle size={24} />}
          color="green"
        />
        <MetricsCard
          title="Failed"
          value={metrics?.records_sunk_failed ?? 0}
          icon={<XCircle size={24} />}
          color="red"
        />
        <MetricsCard
          title="Dead Letter Queue"
          value={metrics?.records_dlq ?? 0}
          icon={<AlertTriangle size={24} />}
          color="amber"
        />
      </div>
    </div>
  );
}