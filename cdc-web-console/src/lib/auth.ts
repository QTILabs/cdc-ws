// export function isAuthenticated(): boolean {
//   return !!localStorage.getItem("jwt_token");
// }

// export function getToken(): string | null {
//   return localStorage.getItem("jwt_token");
// }

// export function getUser(): string | null {
//   return localStorage.getItem("jwt_user");
// }

// export function setAuth(token: string, user: string): void {
//   localStorage.setItem("jwt_token", token);
//   localStorage.setItem("jwt_user", user);
// }

// export function clearAuth(): void {
//   localStorage.removeItem("jwt_token");
//   localStorage.removeItem("jwt_user");
// }
export function isAuthenticated(): boolean {
  if (typeof window === "undefined") return false;
  return !!localStorage.getItem("jwt_token");
}

export function getToken(): string | null {
  if (typeof window === "undefined") return null;
  return localStorage.getItem("jwt_token");
}

export function getUser(): string | null {
  if (typeof window === "undefined") return null;
  return localStorage.getItem("jwt_user");
}

export function setAuth(token: string, user: string): void {
  if (typeof window === "undefined") return;
  localStorage.setItem("jwt_token", token);
  localStorage.setItem("jwt_user", user);
}

export function clearAuth(): void {
  if (typeof window === "undefined") return;
  localStorage.removeItem("jwt_token");
  localStorage.removeItem("jwt_user");
}