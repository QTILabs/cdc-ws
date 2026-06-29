import { apiClient } from './client';

export interface LoginPayload {
  username: string;
  password: string;
}

export interface TokenResponse {
  token: string;
  user: string;
}

export async function loginLocal(payload: LoginPayload): Promise<TokenResponse> {
  const { data } = await apiClient.post<TokenResponse>('/auth/login', payload);
  return data;
}

export function getOAuthLoginUrl(provider: string): string {
  return `/api/auth/oauth2/${provider}/login`;
}