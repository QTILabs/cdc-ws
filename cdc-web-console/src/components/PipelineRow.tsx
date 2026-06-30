import { Component } from "solid-js";
import { Pause, Play, AlertCircle } from "lucide-solid";
import type { PipelineStatus } from "~/lib/api";

interface Props {
  pipeline: PipelineStatus;
  onPause: (name: string) => void;
}

const stateStyles = {
  RUNNING: "bg-green-100 text-green-700",
  PAUSED: "bg-amber-100 text-amber-700",
  ERROR: "bg-red-100 text-red-700",
};

const PipelineRow: Component<Props> = (props) => {
  return (
    <tr class="border-b border-slate-100 hover:bg-slate-50">
      <td class="px-6 py-4">
        <div class="font-medium text-slate-900">{props.pipeline.subscription_name}</div>
        <div class="text-xs text-slate-500 font-mono mt-0.5">
          {props.pipeline.cursor_name}
        </div>
      </td>
      <td class="px-6 py-4 text-sm text-slate-700">{props.pipeline.target_index}</td>
      <td class="px-6 py-4">
        <span
          class={`inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium ${
            stateStyles[props.pipeline.state]
          }`}
        >
          {props.pipeline.state === "ERROR" && <AlertCircle size={12} />}
          {props.pipeline.state}
        </span>
      </td>
      <td class="px-6 py-4 text-right">
        {props.pipeline.state === "RUNNING" && (
          <button
            onClick={() => props.onPause(props.pipeline.subscription_name)}
            class="inline-flex items-center gap-1.5 px-3 py-1.5 text-sm text-amber-700 bg-amber-50 hover:bg-amber-100 rounded-lg transition"
          >
            <Pause size={14} /> Pause
          </button>
        )}
        {props.pipeline.state === "PAUSED" && (
          <button
            disabled
            class="inline-flex items-center gap-1.5 px-3 py-1.5 text-sm text-slate-400 bg-slate-50 rounded-lg cursor-not-allowed"
          >
            <Play size={14} /> Resume
          </button>
        )}
      </td>
    </tr>
  );
};

export default PipelineRow;