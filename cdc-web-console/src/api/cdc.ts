import { apiClient } from './client';

export interface HealthResponse {
  is_healthy: boolean;
  overall_status: 'RUNNING' | 'DEGRADED' | 'DOWN';
  components: Record<string, string>;
}

export interface MetricsResponse {
  records_ingested: number;
  records_sunk_success: number;
  records_sunk_failed: number;
  records_dlq: number;
}

export interface PipelineStatus {
  subscription_name: string;
  target_index: string;
  cursor_name: string;
  state: 'RUNNING' | 'PAUSED' | 'ERROR';
}

export interface ListPipelinesResponse {
  pipelines: PipelineStatus[];
}

export interface ControlResponse {
  success: boolean;
  message: string;
}

export const cdcApi = {
  getHealth: () => apiClient.get<HealthResponse>('/cdc/health').then((r) => r.data),
  getMetrics: () => apiClient.get<MetricsResponse>('/cdc/metrics').then((r) => r.data),
  listPipelines: () => apiClient.get<ListPipelinesResponse>('/cdc/pipelines').then((r) => r.data),
  pausePipeline: (name: string) =>
    apiClient.post<ControlResponse>(`/cdc/pipelines/${name}/pause`).then((r) => r.data),
  reloadPipelines: () => apiClient.post<ControlResponse>('/cdc/pipelines/reload').then((r) => r.data),
};