import { Outlet, NavLink, useNavigate } from 'react-router-dom';
import { useAuth } from '../hooks/useAuth';
import { LayoutDashboard, GitBranch, LogOut } from 'lucide-react';

export default function Layout() {
  const { user, logout } = useAuth();
  const navigate = useNavigate();

  const handleLogout = () => {
    logout();
    void navigate('/login');
  };

  const linkClass = ({ isActive }: { isActive: boolean }) =>
    `flex items-center gap-2 px-4 py-2 rounded-lg transition ${
      isActive ? 'bg-primary-600 text-white' : 'text-slate-700 hover:bg-slate-100'
    }`;

  return (
    <div className="flex h-screen bg-slate-50">
      {/* Sidebar */}
      <aside className="w-64 bg-white border-r border-slate-200 flex flex-col">
        <div className="p-6 border-b border-slate-200">
          <h1 className="text-xl font-bold text-slate-900">CDC Console</h1>
          <p className="text-xs text-slate-500 mt-1">RisingWave • OpenSearch</p>
        </div>
        <nav className="flex-1 p-4 space-y-1">
          <NavLink to="/dashboard" className={linkClass}>
            <LayoutDashboard size={18} /> Dashboard
          </NavLink>
          <NavLink to="/pipelines" className={linkClass}>
            <GitBranch size={18} /> Pipelines
          </NavLink>
        </nav>
        <div className="p-4 border-t border-slate-200">
          <div className="flex items-center justify-between">
            <span className="text-sm text-slate-600 truncate">{user}</span>
            <button onClick={handleLogout} className="text-slate-400 hover:text-red-500">
              <LogOut size={18} />
            </button>
          </div>
        </div>
      </aside>

      {/* Main content */}
      <main className="flex-1 overflow-auto">
        <div className="p-8">
          <Outlet />
        </div>
      </main>
    </div>
  );
}