import { PipelineStatus } from '../api/cdc';
import { Pause, Play, AlertCircle } from 'lucide-react';

interface Props {
  pipeline: PipelineStatus;
  onPause: (name: string) => void;
}

const stateStyles = {
  RUNNING: 'bg-green-100 text-green-700',
  PAUSED: 'bg-amber-100 text-amber-700',
  ERROR: 'bg-red-100 text-red-700',
};

export default function PipelineRow({ pipeline, onPause }: Props) {
  return (
    <tr className="border-b border-slate-100 hover:bg-slate-50">
      <td className="px-6 py-4">
        <div className="font-medium text-slate-900">{pipeline.subscription_name}</div>
        <div className="text-xs text-slate-500 font-mono mt-0.5">{pipeline.cursor_name}</div>
      </td>
      <td className="px-6 py-4 text-sm text-slate-700">{pipeline.target_index}</td>
      <td className="px-6 py-4">
        <span className={`inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium ${stateStyles[pipeline.state]}`}>
          {pipeline.state === 'ERROR' && <AlertCircle size={12} />}
          {pipeline.state}
        </span>
      </td>
      <td className="px-6 py-4 text-right">
        {pipeline.state === 'RUNNING' && (
          <button
            onClick={() => onPause(pipeline.subscription_name)}
            className="inline-flex items-center gap-1.5 px-3 py-1.5 text-sm text-amber-700 bg-amber-50 hover:bg-amber-100 rounded-lg transition"
          >
            <Pause size={14} /> Pause
          </button>
        )}
        {pipeline.state === 'PAUSED' && (
          <button
            disabled
            className="inline-flex items-center gap-1.5 px-3 py-1.5 text-sm text-slate-400 bg-slate-50 rounded-lg cursor-not-allowed"
          >
            <Play size={14} /> Resume
          </button>
        )}
      </td>
    </tr>
  );
}