import React, { useEffect, useState } from "react";
import { Shield, Activity, DollarSign, Server, ShieldAlert, Check } from "lucide-react";

type TrafficLog = {
  id: string;
  ip: string;
  status: "proxied" | "tarpitted";
  time: string;
  latency?: string;
};

export default function App() {
  const [tarpits, setTarpits] = useState(0);
  const [blocked, setBlocked] = useState(0);
  const [logs, setLogs] = useState<TrafficLog[]>([]);
  const [isSimulatingBot, setIsSimulatingBot] = useState(false);

  useEffect(() => {
    const interval = setInterval(() => {
      if (isSimulatingBot) return;
      if (Math.random() > 0.6) {
        const isBot = Math.random() > 0.8;
        const newLog: TrafficLog = {
          id: Math.random().toString(36).substring(7),
          ip: isBot ? \`192.168.1.\${Math.floor(Math.random() * 5)}\` : \`203.0.113.\${Math.floor(Math.random() * 255)}\`,
          status: isBot ? "tarpitted" : "proxied",
          time: new Date().toLocaleTimeString(),
          latency: isBot ? undefined : \`\${Math.floor(Math.random() * 150 + 50)}ms\`,
        };

        if (newLog.status === "tarpitted") {
          setTarpits(p => p + 1);
          setBlocked(p => p + 1);
          setTimeout(() => setTarpits(p => Math.max(0, p - 1)), Math.random() * 5000 + 3000);
        }
        setLogs(prev => [newLog, ...prev].slice(0, 8));
      }
    }, 800);
    return () => clearInterval(interval);
  }, [isSimulatingBot]);

  const simulateBotAttack = () => {
    if (isSimulatingBot) return;
    setIsSimulatingBot(true);
    let sent = 0;
    const attackInterval = setInterval(() => {
      const newLog: TrafficLog = {
        id: Math.random().toString(36).substring(7),
        ip: \`10.0.0.\${Math.floor(Math.random() * 3)}\`,
        status: "tarpitted",
        time: new Date().toLocaleTimeString(),
      };
      setTarpits(p => p + 1);
      setBlocked(p => p + 1);
      setLogs(prev => [newLog, ...prev].slice(0, 8));
      sent++;
      if (sent >= 20) {
        clearInterval(attackInterval);
        setIsSimulatingBot(false);
        setTimeout(() => setTarpits(0), 8000);
      }
    }, 150);
  };

  return (
    <div className="min-h-screen bg-[#0a0a0a] text-zinc-300 font-sans p-6 md:p-12">
      <div className="max-w-6xl mx-auto space-y-8">
        <header className="flex flex-col md:flex-row md:items-center justify-between gap-4 border-b border-zinc-800 pb-6">
          <div>
            <h1 className="text-3xl font-bold text-white flex items-center gap-3">
              <Shield className="w-8 h-8 text-emerald-500" />
              Drop-In Shield Dashboard
            </h1>
            <p className="text-zinc-500 mt-1">Live monitoring for your proxy</p>
          </div>
          <button 
            onClick={simulateBotAttack}
            disabled={isSimulatingBot}
            className="px-4 py-2 bg-rose-500/10 text-rose-500 rounded-lg border border-rose-500/20 text-sm font-medium disabled:opacity-50"
          >
            {isSimulatingBot ? "Attack in Progress..." : "Simulate Attack"}
          </button>
        </header>

        <div className="grid grid-cols-1 md:grid-cols-4 gap-6">
          <div className="bg-zinc-900 border border-zinc-800 rounded-2xl p-6">
            <p className="text-sm font-medium text-zinc-500">Active Tarpits</p>
            <p className="text-4xl font-bold text-white mt-2">{tarpits}</p>
          </div>
          <div className="bg-zinc-900 border border-zinc-800 rounded-2xl p-6">
            <p className="text-sm font-medium text-zinc-500">Total Blocked</p>
            <p className="text-4xl font-bold text-white mt-2">{blocked}</p>
          </div>
          <div className="bg-zinc-900 border border-zinc-800 rounded-2xl p-6">
            <p className="text-sm font-medium text-zinc-500">Credits Saved</p>
            <p className="text-4xl font-bold text-white mt-2">${(blocked * 0.002).toFixed(2)}</p>
          </div>
          <div className="bg-zinc-900 border border-zinc-800 rounded-2xl p-6">
            <p className="text-sm font-medium text-zinc-500">CPU Overhead</p>
            <p className="text-4xl font-bold text-white mt-2">0.1%</p>
          </div>
        </div>

        <div className="bg-zinc-900 border border-zinc-800 rounded-2xl overflow-hidden">
          <div className="px-6 py-5 border-b border-zinc-800">
            <h2 className="text-lg font-medium text-white flex items-center gap-2">
              <Activity className="w-5 h-5 text-zinc-400" />
              Live Traffic Log
            </h2>
          </div>
          <table className="w-full text-left text-sm whitespace-nowrap">
            <thead className="bg-zinc-900/50 text-zinc-500 border-b border-zinc-800">
              <tr>
                <th className="px-6 py-4">Time</th>
                <th className="px-6 py-4">IP Address</th>
                <th className="px-6 py-4">Status</th>
                <th className="px-6 py-4">Latency</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-zinc-800/50">
              {logs.map((log) => (
                <tr key={log.id}>
                  <td className="px-6 py-4 text-zinc-400">{log.time}</td>
                  <td className="px-6 py-4 font-mono">{log.ip}</td>
                  <td className="px-6 py-4">
                    {log.status === "proxied" ? (
                      <span className="text-emerald-400 bg-emerald-500/10 px-2 py-1 rounded-full text-xs">Allowed</span>
                    ) : (
                      <span className="text-rose-400 bg-rose-500/10 px-2 py-1 rounded-full text-xs">Tarpitted</span>
                    )}
                  </td>
                  <td className="px-6 py-4 text-zinc-400">{log.latency || "Holding..."}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}
