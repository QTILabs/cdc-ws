import { createQuery } from "@tanstack/solid-query";
import ProtectedRoute from "~/components/ProtectedRoute";
import Layout from "~/components/Layout";
import MetricsCard from "~/components/MetricsCard";
import { api } from "~/lib/api";
import { Activity, CheckCircle, XCircle, AlertTriangle } from "lucide-solid";

export default function Dashboard() {
  return (
    <ProtectedRoute>
      <Layout>
        <div>
          <div class="mb-8">
            <h1 class="text-2xl font-bold text-slate-900">Dashboard</h1>
            <p class="text-sm text-slate-500 mt-1">Real-time CDC daemon monitoring</p>
          </div>

          <HealthBanner />

          <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
            <MetricsQuery />
          </div>
        </div>
      </Layout>
    </ProtectedRoute>
  );
}

function HealthBanner() {
  const health = createQuery(() => ({
    queryKey: ["health"],
    queryFn: api.getHealth,
    refetchInterval: 5000,
  }));

  return (
    <>
      {health.data && (
        <div
          class={`mb-6 p-4 rounded-xl border ${
            health.data.is_healthy
              ? "bg-green-50 border-green-200 text-green-800"
              : "bg-red-50 border-red-200 text-red-800"
          }`}
        >
          <div class="flex items-center gap-3">
            {health.data.is_healthy ? (
              <CheckCircle size={20} />
            ) : (
              <AlertTriangle size={20} />
            )}
            <div>
              <p class="font-semibold">System Status: {health.data.overall_status}</p>
              <p class="text-sm opacity-80">
                {Object.entries(health.data.components)
                  .map(([k, v]) => `${k}: ${v}`)
                  .join(" • ")}
              </p>
            </div>
          </div>
        </div>
      )}
    </>
  );
}

function MetricsQuery() {
  const metrics = createQuery(() => ({
    queryKey: ["metrics"],
    queryFn: api.getMetrics,
    refetchInterval: 3000,
  }));

  return (
    <>
      <MetricsCard
        title="Records Ingested"
        value={metrics.data?.records_ingested ?? 0}
        icon={Activity}
        color="blue"
      />
      <MetricsCard
        title="Successfully Sunk"
        value={metrics.data?.records_sunk_success ?? 0}
        icon={CheckCircle}
        color="green"
      />
      <MetricsCard
        title="Failed"
        value={metrics.data?.records_sunk_failed ?? 0}
        icon={XCircle}
        color="red"
      />
      <MetricsCard
        title="Dead Letter Queue"
        value={metrics.data?.records_dlq ?? 0}
        icon={AlertTriangle}
        color="amber"
      />

      <SinkMetricsSection sinkMetrics={metrics.data?.sink_metrics} />
    </>
  );
}

function SinkMetricsSection(props: {
  sinkMetrics?: Record<string, { sunk_success: number; sunk_failed: number }>;
}) {
  const entries = () => Object.entries(props.sinkMetrics ?? {});

  return (
    <>
      {entries().length > 0 && (
        <div class="mt-8 md:col-span-2 lg:col-span-4">
          <div class="mb-4">
            <h2 class="text-lg font-semibold text-slate-900">Per-Sink Metrics</h2>
            <p class="text-sm text-slate-500 mt-1">
              Success and failure counters by sink type
            </p>
          </div>
          <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            {entries().map(([sink, values]) => (
              <div class="bg-white rounded-xl border border-slate-200 p-5">
                <div class="flex items-center justify-between mb-4">
                  <h3 class="text-sm font-semibold text-slate-700 uppercase tracking-wide">
                    {formatSinkName(sink)}
                  </h3>
                </div>
                <div class="grid grid-cols-2 gap-3">
                  <div class="rounded-lg bg-green-50 p-3">
                    <p class="text-xs text-green-700">Sunk Success</p>
                    <p class="text-xl font-bold text-green-800">
                      {values.sunk_success.toLocaleString()}
                    </p>
                  </div>
                  <div class="rounded-lg bg-red-50 p-3">
                    <p class="text-xs text-red-700">Sunk Failed</p>
                    <p class="text-xl font-bold text-red-800">
                      {values.sunk_failed.toLocaleString()}
                    </p>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </>
  );
}

function formatSinkName(name: string): string {
  return name
    .split(/[_-]/g)
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}