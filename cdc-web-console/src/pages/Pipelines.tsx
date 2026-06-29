import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { cdcApi } from '../api/cdc';
import PipelineRow from '../components/PipelineRow';
import { GitBranch, RefreshCw } from 'lucide-react';

export default function Pipelines() {
  const queryClient = useQueryClient();

  const { data, isLoading } = useQuery({
    queryKey: ['pipelines'],
    queryFn: cdcApi.listPipelines,
    refetchInterval: 5000,
  });

  const pauseMutation = useMutation({
    mutationFn: cdcApi.pausePipeline,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['pipelines'] }),
  });

  const reloadMutation = useMutation({
    mutationFn: cdcApi.reloadPipelines,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['pipelines'] }),
  });

  return (
    <div>
      <div className="mb-8">
        <div className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
          <div>
            <h1 className="text-2xl font-bold text-slate-900">Pipelines</h1>
            <p className="text-sm text-slate-500 mt-1">Manage your CDC subscriptions</p>
          </div>
          <button
            onClick={() => reloadMutation.mutate()}
            disabled={reloadMutation.isPending}
            className="inline-flex items-center justify-center gap-2 px-4 py-2 text-sm font-medium text-slate-700 bg-white border border-slate-200 rounded-xl hover:bg-slate-50 disabled:opacity-60 disabled:cursor-not-allowed transition"
          >
            <RefreshCw size={14} className={reloadMutation.isPending ? 'animate-spin' : ''} />
            Reload daemon
          </button>
        </div>
      </div>

      <div className="bg-white rounded-xl border border-slate-200 overflow-hidden">
        {isLoading ? (
          <div className="p-12 text-center text-slate-500">Loading pipelines...</div>
        ) : !data?.pipelines.length ? (
          <div className="p-12 text-center">
            <GitBranch className="mx-auto text-slate-300" size={48} />
            <p className="mt-4 text-slate-500">No pipelines configured</p>
          </div>
        ) : (
          <table className="w-full">
            <thead className="bg-slate-50 border-b border-slate-200">
              <tr>
                <th className="px-6 py-3 text-left text-xs font-semibold text-slate-600 uppercase">
                  Subscription
                </th>
                <th className="px-6 py-3 text-left text-xs font-semibold text-slate-600 uppercase">
                  Target Index
                </th>
                <th className="px-6 py-3 text-left text-xs font-semibold text-slate-600 uppercase">
                  State
                </th>
                <th className="px-6 py-3 text-right text-xs font-semibold text-slate-600 uppercase">
                  Actions
                </th>
              </tr>
            </thead>
            <tbody>
              {data.pipelines.map((p) => (
                <PipelineRow
                  key={p.subscription_name}
                  pipeline={p}
                  onPause={(name) => pauseMutation.mutate(name)}
                />
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}