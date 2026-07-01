import { createContext, useContext, createSignal, ParentProps } from "solid-js";
import { setAuth, clearAuth, getToken, getUser } from "~/lib/auth";

interface AuthContextType {
  token: () => string | null;
  user: () => string | null;
  isAuthenticated: () => boolean;
  login: (token: string, user: string) => void;
  logout: () => void;
}

const AuthContext = createContext<AuthContextType>();

export function AuthProvider(props: ParentProps) {
  const [token, setToken] = createSignal<string | null>(getToken());
  const [user, setUser] = createSignal<string | null>(getUser());

  const login = (newToken: string, newUser: string) => {
    setAuth(newToken, newUser);
    setToken(newToken);
    setUser(newUser);
  };

  const logout = () => {
    clearAuth();
    setToken(null);
    setUser(null);
  };

  const value: AuthContextType = {
    token,
    user,
    isAuthenticated: () => !!token(),
    login,
    logout,
  };

  return <AuthContext.Provider value={value}>{props.children}</AuthContext.Provider>;
}

export function useAuth() {
  const context = useContext(AuthContext);
  if (!context) {
    throw new Error("useAuth must be used within AuthProvider");
  }
  return context;
}