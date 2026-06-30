import { Component } from "solid-js";

interface Props {
  title: string;
  value: number;
  icon: any;
  color: "blue" | "green" | "red" | "amber";
}

const colorMap = {
  blue: "bg-blue-50 text-blue-600",
  green: "bg-green-50 text-green-600",
  red: "bg-red-50 text-red-600",
  amber: "bg-amber-50 text-amber-600",
};

const MetricsCard: Component<Props> = (props) => {
  const Icon = props.icon;

  return (
    <div class="bg-white rounded-xl border border-slate-200 p-6">
      <div class="flex items-center justify-between">
        <div>
          <p class="text-sm text-slate-500">{props.title}</p>
          <p class="text-3xl font-bold text-slate-900 mt-1">
            {props.value.toLocaleString()}
          </p>
        </div>
        <div class={`p-3 rounded-lg ${colorMap[props.color]}`}>
          <Icon size={24} />
        </div>
      </div>
    </div>
  );
};

export default MetricsCard;