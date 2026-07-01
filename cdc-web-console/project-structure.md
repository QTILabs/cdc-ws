# Project Structure
```text
cdc-web-console/
├── package.json
├── vite.config.ts
├── tsconfig.json
├── app.config.ts
├── tailwind.config
├── postcss.config
├── index.html
└── src/
    ├── app.css
    ├── app.tsx
    ├── entry-client.tsx
    ├── entry-server.tsx
    ├── routes/
    │   ├── layout.tsx
    │   ├── index.tsx
    │   ├── login.tsx
    │   ├── auth/
    │   │   └── callback.tsx
    │   ├── dashboard.tsx
    │   └── pipelines.tsx
    ├── components/
    │   ├── Layout.tsx
    │   ├── ProtectedRoute.tsx

    # Feature Notes
    - `src/routes/dashboard.tsx` renders aggregate metrics and per-sink counters from `/api/cdc/metrics` (`sink_metrics`).
    - `src/routes/pipelines.tsx` lists active pipelines and supports pause actions.
    - `src/routes/login.tsx` supports local login and OAuth login (GitHub/OIDC).

    │   ├── MetricsCard.tsx
    │   └── PipelineRow.tsx
    ├── lib/
    │   ├── api.ts
    │   └── auth.ts
    └── context/
        └── AuthContext.tsx
```
# Install dependencies

    # Verify production build
    ```bash
    pnpm run build
    ```
```bash
    # Run production server locally
pnpm install
    pnpm run start
# Start development server (runs on http://localhost:3000)
```bash
pnpm run build
```
# Start production server
```bash
pnpm start
```