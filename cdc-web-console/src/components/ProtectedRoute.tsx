import { Navigate } from "@solidjs/router";
import { ParentProps } from "solid-js";
import { useAuth } from "~/context/AuthContext";

export default function ProtectedRoute(props: ParentProps) {
  const { isAuthenticated } = useAuth();

  return isAuthenticated() ? <>{props.children}</> : <Navigate href="/login" />;
}