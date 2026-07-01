import { defineConfig } from "vite";
import { nitroV2Plugin as nitro } from "@solidjs/vite-plugin-nitro-2";
import tailwindcss from "@tailwindcss/vite";
import { solidStart } from "@solidjs/start/config";

export default defineConfig({
  plugins: [solidStart(),
    nitro(),
    tailwindcss(),
  ],
  server: {
    port: 3000,
  },
});
