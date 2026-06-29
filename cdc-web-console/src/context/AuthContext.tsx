import { createContext, useEffect, useState, ReactNode } from 'react';
import { apiClient } from '../api/client';

interface AuthContextType {
  token: string | null;
  user: string | null;
  isAuthenticated: boolean;
  isLoading: boolean;
  login: (token: string, user: string) => void;
  logout: () => void;
}

export const AuthContext = createContext<AuthContextType | null>(null);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [token, setToken] = useState<string | null>(localStorage.getItem('jwt_token'));
  const [user, setUser] = useState<string | null>(localStorage.getItem('jwt_user'));
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    if (token) {
      apiClient.defaults.headers.common['Authorization'] = `Bearer ${token}`;
    } else {
      delete apiClient.defaults.headers.common['Authorization'];
    }
    setIsLoading(false);
  }, [token]);

  const login = (newToken: string, newUser: string) => {
    localStorage.setItem('jwt_token', newToken);
    localStorage.setItem('jwt_user', newUser);
    setToken(newToken);
    setUser(newUser);
  };

  const logout = () => {
    localStorage.removeItem('jwt_token');
    localStorage.removeItem('jwt_user');
    setToken(null);
    setUser(null);
  };

  return (
    <AuthContext.Provider
      value={{
        token,
        user,
        isAuthenticated: !!token,
        isLoading,
        login,
        logout,
      }}
    >
      {children}
    </AuthContext.Provider>
  );
}