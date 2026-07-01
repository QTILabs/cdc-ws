import { Router } from "@solidjs/router";
import { FileRoutes } from "@solidjs/start/router";
import { Suspense } from "solid-js";
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import { AuthProvider } from "./context/AuthContext";
import "./app.css";

export default function App() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        refetchOnWindowFocus: false,
        retry: 1,
        staleTime: 5000,
      },
    },
  });

  return (
    <QueryClientProvider client={queryClient}>
      <Router
        root={(props) => (
          <AuthProvider>
            <Suspense>{props.children}</Suspense>
          </AuthProvider>
        )}
      >
        <FileRoutes />
      </Router>
    </QueryClientProvider>
  );
}