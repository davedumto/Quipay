import {
  PieChart,
  Pie,
  Cell,
  Tooltip,
  Legend,
  ResponsiveContainer,
} from "recharts";

interface Datapoint {
  name: string;
  value: number;
}

const COLORS = ["#10b981", "#f59e0b", "#ef4444"];

const tooltipStyle = {
  backgroundColor: "#0f172a",
  border: "1px solid rgba(99,102,241,0.2)",
  borderRadius: "0.75rem",
  color: "#e2e8f0",
  fontSize: "0.8rem",
};

export function StreamStatusChart({ data }: { data: Datapoint[] }) {
  const total = data.reduce((s, d) => s + d.value, 0);
  return (
    <ResponsiveContainer width="100%" height={240}>
      <PieChart>
        <Pie
          data={data}
          cx="50%"
          cy="45%"
          innerRadius={65}
          outerRadius={95}
          paddingAngle={3}
          dataKey="value"
        >
          {data.map((_, i) => (
            <Cell key={i} fill={COLORS[i % COLORS.length]} />
          ))}
        </Pie>
        <Tooltip
          contentStyle={tooltipStyle}
          formatter={(v: number) => [
            `${v} (${total > 0 ? ((v / total) * 100).toFixed(1) : 0}%)`,
          ]}
        />
        <Legend wrapperStyle={{ fontSize: "0.8rem", color: "#94a3b8" }} />
      </PieChart>
    </ResponsiveContainer>
  );
}
