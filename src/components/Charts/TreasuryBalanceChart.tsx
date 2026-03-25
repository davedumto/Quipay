import {
  AreaChart,
  Area,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ResponsiveContainer,
} from "recharts";

interface Datapoint {
  month: string;
  balance: number;
  payouts: number;
}

const tooltipStyle = {
  backgroundColor: "#0f172a",
  border: "1px solid rgba(99,102,241,0.2)",
  borderRadius: "0.75rem",
  color: "#e2e8f0",
  fontSize: "0.8rem",
};

export function TreasuryBalanceChart({ data }: { data: Datapoint[] }) {
  return (
    <ResponsiveContainer width="100%" height={240}>
      <AreaChart data={data} margin={{ top: 10, right: 16, left: 0, bottom: 0 }}>
        <defs>
          <linearGradient id="tbc-balance" x1="0" y1="0" x2="0" y2="1">
            <stop offset="5%" stopColor="#10b981" stopOpacity={0.3} />
            <stop offset="95%" stopColor="#10b981" stopOpacity={0.02} />
          </linearGradient>
        </defs>
        <CartesianGrid
          strokeDasharray="3 3"
          stroke="rgba(99,102,241,0.1)"
          vertical={false}
        />
        <XAxis
          dataKey="month"
          tick={{ fill: "#64748b", fontSize: 12 }}
          axisLine={false}
          tickLine={false}
        />
        <YAxis
          tick={{ fill: "#64748b", fontSize: 12 }}
          axisLine={false}
          tickLine={false}
          tickFormatter={(v: number) => `$${(v / 1000).toFixed(0)}k`}
        />
        <Tooltip
          contentStyle={tooltipStyle}
          formatter={(v: number) => [`$${v.toLocaleString()} USDC`]}
        />
        <Legend wrapperStyle={{ fontSize: "0.8rem", color: "#94a3b8" }} />
        <Area
          type="monotone"
          dataKey="balance"
          name="Balance"
          stroke="#10b981"
          strokeWidth={2}
          fill="url(#tbc-balance)"
        />
        <Area
          type="monotone"
          dataKey="payouts"
          name="Payouts"
          stroke="#8b5cf6"
          strokeWidth={2}
          fill="none"
          strokeDasharray="4 4"
        />
      </AreaChart>
    </ResponsiveContainer>
  );
}
