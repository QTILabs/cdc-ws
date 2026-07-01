import { createSignal } from "solid-js";
import { useNavigate } from "@solidjs/router";
import { useAuth } from "~/context/AuthContext";
import { api } from "~/lib/api";
import { Github, Mail } from "lucide-solid";

export default function Login() {
  const [username, setUsername] = createSignal("");
  const [password, setPassword] = createSignal("");
  const [error, setError] = createSignal("");
  const [loading, setLoading] = createSignal(false);
  const { login } = useAuth();
  const navigate = useNavigate();

  const handleLocalLogin = async (e: Event) => {
    e.preventDefault();
    setError("");
    setLoading(true);

    try {
      const res = await api.loginLocal(username(), password());
      login(res.token, res.user);
      navigate("/dashboard");
    } catch (err: any) {
      setError(err.message || "Login failed");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div class="min-h-screen flex items-center justify-center bg-linear-to-br from-slate-50 to-slate-100">
      <div class="w-full max-w-md bg-white rounded-2xl shadow-xl p-8">
        <img src="/img/logo.png" class="mb-5" />
        <div class="text-center mb-8">
          <h1 class="text-2xl font-bold text-slate-900">CDC Console</h1>
          <p class="text-sm text-slate-500 mt-1">Sign in to manage your pipelines</p>
        </div>

        <form onSubmit={handleLocalLogin} class="space-y-4">
          <div>
            <label class="block text-sm font-medium text-slate-700 mb-1">Username</label>
            <input
              type="text"
              value={username()}
              onInput={(e) => setUsername(e.currentTarget.value)}
              class="w-full px-4 py-2 border border-slate-300 rounded-lg focus:ring-2 focus:ring-primary-500 focus:border-transparent"
              required
            />
          </div>
          <div>
            <label class="block text-sm font-medium text-slate-700 mb-1">Password</label>
            <input
              type="password"
              value={password()}
              onInput={(e) => setPassword(e.currentTarget.value)}
              class="w-full px-4 py-2 border border-slate-300 rounded-lg focus:ring-2 focus:ring-primary-500 focus:border-transparent"
              required
            />
          </div>
          {error() && <p class="text-sm text-red-600">{error()}</p>}
          <button
            type="submit"
            disabled={loading()}
            class="w-full py-2.5 bg-primary hover:bg-button-primary-hover text-white cursor-pointer rounded-lg font-medium transition disabled:opacity-50"
          >
            {loading() ? "Signing in..." : "Sign in"}
          </button>
        </form>

        <div class="relative my-6">
          <div class="absolute inset-0 flex items-center">
            <div class="w-full border-t border-slate-200" />
          </div>
          <div class="relative flex justify-center text-xs">
            <span class="px-2 bg-white text-slate-500">Or continue with</span>
          </div>
        </div>

        <div class="grid grid-cols-2 gap-3">
          <a
            href={api.getOAuthLoginUrl("github")}
            class="flex items-center justify-center gap-2 py-2.5 border border-slate-300 rounded-lg hover:bg-slate-50 transition"
          >
            <Github size={18} /> GitHub
          </a>
          <a
            href={api.getOAuthLoginUrl("oidc")}
            class="flex items-center justify-center gap-2 py-2.5 border border-slate-300 rounded-lg hover:bg-slate-50 transition"
          >
            <Mail size={18} /> SSO
          </a>
        </div>
      </div>
    </div>
  );
}