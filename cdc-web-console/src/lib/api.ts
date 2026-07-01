const API_BASE = "http://localhost:8080/api";

export interface HealthResponse {
  is_healthy: boolean;
  overall_status: "RUNNING" | "DEGRADED" | "DOWN";
  components: Record<string, string>;
}

export interface MetricsResponse {
  records_ingested: number;
  records_sunk_success: number;
  records_sunk_failed: number;
  records_dlq: number;
  sink_metrics?: Record<string, SinkMetrics>;
}

export interface SinkMetrics {
  sunk_success: number;
  sunk_failed: number;
}

export interface PipelineStatus {
  subscription_name: string;
  target_index: string;
  cursor_name: string;
  state: "RUNNING" | "PAUSED" | "ERROR";
}

export interface ListPipelinesResponse {
  pipelines: PipelineStatus[];
}

export interface TokenResponse {
  token: string;
  user: string;
}

// async function fetchWithAuth<T>(
//   endpoint: string,
//   options: RequestInit = {}
// ): Promise<T> {
//   const token = localStorage.getItem("jwt_token");
//   const headers: HeadersInit = {
//     "Content-Type": "application/json",
//     ...options.headers,
//   };

//   if (token) {
//     headers.Authorization = `Bearer ${token}`;
//   }

//   const response = await fetch(`${API_BASE}${endpoint}`, {
//     ...options,
//     headers,
//   });

//   if (response.status === 401) {
//     localStorage.removeItem("jwt_token");
//     localStorage.removeItem("jwt_user");
//     window.location.href = "/login";
//     throw new Error("Unauthorized");
//   }

//   if (!response.ok) {
//     throw new Error(`HTTP ${response.status}`);
//   }

//   return response.json();
// }
async function fetchWithAuth<T>(
  endpoint: string,
  options: RequestInit = {}
): Promise<T> {
  const token = localStorage.getItem("jwt_token");

  const headers: Record<string, string> = {
    "Content-Type": "application/json",
  };

  if (options.headers) {
    new Headers(options.headers).forEach((value, key) => {
      headers[key] = value;
    });
  }

  if (token) {
    headers.Authorization = `Bearer ${token}`;
  }

  const response = await fetch(`${API_BASE}${endpoint}`, {
    ...options,
    headers,
  });

  if (response.status === 401) {
    localStorage.removeItem("jwt_token");
    localStorage.removeItem("jwt_user");
    window.location.href = "/login";
    throw new Error("Unauthorized");
  }

  if (!response.ok) {
    throw new Error(`HTTP ${response.status}`);
  }

  return response.json();
}

export const api = {
  // Auth
  loginLocal: async (username: string, password: string): Promise<TokenResponse> => {
    const response = await fetch(`${API_BASE}/auth/login`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ username, password }),
    });

    if (!response.ok) {
      throw new Error("Invalid credentials");
    }

    return response.json();
  },

  getOAuthLoginUrl: (provider: string): string => {
    return `${API_BASE}/auth/oauth2/${provider}/login`;
  },

  // CDC
  getHealth: (): Promise<HealthResponse> => {
    return fetchWithAuth<HealthResponse>("/cdc/health");
  },

  getMetrics: (): Promise<MetricsResponse> => {
    return fetchWithAuth<MetricsResponse>("/cdc/metrics");
  },

  listPipelines: (): Promise<ListPipelinesResponse> => {
    return fetchWithAuth<ListPipelinesResponse>("/cdc/pipelines");
  },

  pausePipeline: (name: string): Promise<{ success: boolean; message: string }> => {
    return fetchWithAuth(`/cdc/pipelines/${name}/pause`, { method: "POST" });
  },
};