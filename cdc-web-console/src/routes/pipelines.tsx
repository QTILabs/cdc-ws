import { createQuery, createMutation, useQueryClient } from "@tanstack/solid-query";
import ProtectedRoute from "~/components/ProtectedRoute";
import Layout from "~/components/Layout";
import PipelineRow from "~/components/PipelineRow";
import { api } from "~/lib/api";
import { GitBranch } from "lucide-solid";

export default function Pipelines() {
  const queryClient = useQueryClient();

  const pipelines = createQuery(() => ({
    queryKey: ["pipelines"],
    queryFn: api.listPipelines,
    refetchInterval: 5000,
  }));

  const pauseMutation = createMutation(() => ({
    mutationFn: api.pausePipeline,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["pipelines"] });
    },
  }));

  return (
    <ProtectedRoute>
      <Layout>
        <div>
          <div class="mb-8">
            <h1 class="text-2xl font-bold text-slate-900">Pipelines</h1>
            <p class="text-sm text-slate-500 mt-1">Manage your CDC subscriptions</p>
          </div>

          <div class="bg-white rounded-xl border border-slate-200 overflow-hidden">
            {pipelines.isLoading ? (
              <div class="p-12 text-center text-slate-500">Loading pipelines...</div>
            ) : !pipelines.data?.pipelines.length ? (
              <div class="p-12 text-center">
                <GitBranch class="mx-auto text-slate-300" size={48} />
                <p class="mt-4 text-slate-500">No pipelines configured</p>
              </div>
            ) : (
              <table class="w-full">
                <thead class="bg-slate-50 border-b border-slate-200">
                  <tr>
                    <th class="px-6 py-3 text-left text-xs font-semibold text-slate-600 uppercase">
                      Subscription
                    </th>
                    <th class="px-6 py-3 text-left text-xs font-semibold text-slate-600 uppercase">
                      Target Index
                    </th>
                    <th class="px-6 py-3 text-left text-xs font-semibold text-slate-600 uppercase">
                      State
                    </th>
                    <th class="px-6 py-3 text-right text-xs font-semibold text-slate-600 uppercase">
                      Actions
                    </th>
                  </tr>
                </thead>
                <tbody>
                  {pipelines.data.pipelines.map((p) => (
                    <PipelineRow
                      pipeline={p}
                      onPause={(name) => pauseMutation.mutate(name)}
                    />
                  ))}
                </tbody>
              </table>
            )}
          </div>
        </div>
      </Layout>
    </ProtectedRoute>
  );
}