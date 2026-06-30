import { ParentProps } from "solid-js";
import { useNavigate, A } from "@solidjs/router";
import { useAuth } from "~/context/AuthContext";
import { LayoutDashboard, GitBranch, LogOut } from "lucide-solid";

export default function Layout(props: ParentProps) {
  const { user, logout } = useAuth();
  const navigate = useNavigate();

  const handleLogout = () => {
    logout();
    navigate("/login");
  };

  const linkClass = (path: string) => {
    const currentPath = window.location.pathname;
    const isActive = currentPath === path || currentPath.startsWith(path + "/");
    return `flex items-center gap-2 px-4 py-2 rounded-lg transition ${
      isActive ? "bg-primary-600 text-white" : "text-slate-700 hover:bg-slate-100"
    }`;
  };

  return (
    <div class="flex h-screen bg-slate-50">
      {/* Sidebar */}
      <aside class="w-64 bg-white border-r border-slate-200 flex flex-col">
        <div class="p-6 border-b border-slate-200">
          <h1 class="text-xl font-bold text-slate-900">CDC Console</h1>
          <p class="text-xs text-slate-500 mt-1">RisingWave • OpenSearch • Qdrant</p>
        </div>
        <nav class="flex-1 p-4 space-y-1">
          <A href="/dashboard" class={linkClass("/dashboard")}>
            <LayoutDashboard size={18} /> Dashboard
          </A>
          <A href="/pipelines" class={linkClass("/pipelines")}>
            <GitBranch size={18} /> Pipelines
          </A>
        </nav>
        <div class="p-4 border-t border-slate-200">
          <div class="flex items-center justify-between">
            <span class="text-sm text-slate-600 truncate">{user()}</span>
            <button onClick={handleLogout} class="text-slate-400 hover:text-red-500">
              <LogOut size={18} />
            </button>
          </div>
        </div>
      </aside>

      {/* Main content */}
      <main class="flex-1 overflow-auto">
        <div class="p-8">{props.children}</div>
      </main>
    </div>
  );
}