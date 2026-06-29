import { useEffect } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { useAuth } from '../hooks/useAuth';
import { apiClient } from '../api/client';

export default function OAuthCallback() {
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  const { login } = useAuth();

  useEffect(() => {
    const code = searchParams.get('code');
    const state = searchParams.get('state');
    const provider = searchParams.get('provider') || 'github';

    if (!code || !state) {
      void navigate('/login');
      return;
    }

    void apiClient
      .get(`/auth/oauth2/${provider}/callback?code=${code}&state=${state}`)
      .then((res) => {
        login(res.data.token, res.data.user);
        void navigate('/dashboard');
      })
      .catch(() => navigate('/login'));
  }, [searchParams, navigate, login]);

  return (
    <div className="flex h-screen items-center justify-center">
      <div className="text-center">
        <div className="h-12 w-12 animate-spin rounded-full border-4 border-primary-500 border-t-transparent mx-auto" />
        <p className="mt-4 text-slate-600">Completing sign-in...</p>
      </div>
    </div>
  );
}