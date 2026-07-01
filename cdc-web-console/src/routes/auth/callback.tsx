import { onMount } from "solid-js";
import { useNavigate, useSearchParams } from "@solidjs/router";
import { useAuth } from "~/context/AuthContext";

export default function OAuthCallback() {
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  const { login } = useAuth();

  onMount(async () => {
    const code = searchParams.code;
    const state = searchParams.state;
    const provider = searchParams.provider || "github";

    if (!code || !state) {
      navigate("/login");
      return;
    }

    try {
      const response = await fetch(
        `/api/auth/oauth2/${provider}/callback?code=${code}&state=${state}`
      );
      const data = await response.json();
      login(data.token, data.user);
      navigate("/dashboard");
    } catch (err) {
      navigate("/login");
    }
  });

  return (
    <div class="flex h-screen items-center justify-center">
      <div class="text-center">
        <div class="h-12 w-12 animate-spin rounded-full border-4 border-primary-500 border-t-transparent mx-auto" />
        <p class="mt-4 text-slate-600">Completing sign-in...</p>
      </div>
    </div>
  );
}