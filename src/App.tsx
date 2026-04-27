import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { downloadDir } from "@tauri-apps/api/path";
import { Button } from "@/components/ui/button";
import {
  Download as DownloadIcon,
  Plus,
  Activity,
  Settings,
  FolderOpen,
  Pause,
  Trash2,
  Zap,
  Globe,
  HardDrive,
  Clock,
} from "lucide-react";
import { cn } from "@/lib/utils";

// ─── Types ────────────────────────────────────────────────────────────────────
export interface Download {
  id: string;
  url: string;
  filename: string;
  save_path: string;
  total_bytes: number;
  downloaded_bytes: number;
  status: string;
  created_at: string;
  // Dynamic UI properties mapped from the live event stream
  _live_downloaded?: number;
  _live_speed?: number;
  // Optimistic flag — true while we're waiting for the real DB row
  _optimistic?: boolean;
}

export interface Chunk {
  id: string;
  download_id: string;
  start_byte: number;
  end_byte: number;
  downloaded_bytes: number;
  status: string;
}

export interface ProgressPayload {
  download_id: string;
  chunk_id: string;
  downloaded_bytes: number;  // maps from Rust i64
  speed_bytes_per_sec: number; // maps from Rust f64
}

// ─── Sidebar Nav Item ─────────────────────────────────────────────────────────
function NavItem({
  icon: Icon,
  label,
  active,
  onClick,
}: {
  icon: React.ElementType;
  label: string;
  active?: boolean;
  onClick?: () => void;
}) {
  return (
    <button
      onClick={onClick}
      className={cn(
        "flex items-center gap-3 w-full px-3 py-2.5 rounded-lg text-sm font-medium transition-all duration-200",
        active
          ? "bg-primary/20 text-primary border border-primary/30 shadow-sm shadow-primary/10"
          : "text-muted-foreground hover:text-foreground hover:bg-accent"
      )}
    >
      <Icon size={16} />
      {label}
    </button>
  );
}

// ─── Status Badge ─────────────────────────────────────────────────────────────
function StatusBadge({ status }: { status: string }) {
  const map: Record<string, { label: string; cls: string }> = {
    pending: { label: "Pending", cls: "bg-yellow-500/20 text-yellow-400 border-yellow-500/30" },
    downloading: { label: "Downloading", cls: "bg-blue-500/20 text-blue-400 border-blue-500/30 animate-pulse" },
    paused: { label: "Paused", cls: "bg-orange-500/20 text-orange-400 border-orange-500/30" },
    completed: { label: "Done", cls: "bg-green-500/20 text-green-400 border-green-500/30" },
    failed: { label: "Failed", cls: "bg-red-500/20 text-red-400 border-red-500/30" },
  };
  const fallback = { label: status, cls: "bg-gray-500/20 text-gray-400 border-gray-500/30" };
  const { label, cls } = map[status.toLowerCase()] || fallback;

  return (
    <span className={cn("text-xs font-medium px-2 py-0.5 rounded-full border", cls)}>
      {label}
    </span>
  );
}

// ─── Progress Bar ─────────────────────────────────────────────────────────────
function ProgressBar({ value }: { value: number }) {
  return (
    <div className="h-1.5 w-full bg-secondary rounded-full overflow-hidden">
      <div
        className="h-full progress-bar-fill rounded-full transition-all duration-[50ms] ease-linear"
        style={{ width: `${Math.min(100, Math.max(0, value))}%` }}
      />
    </div>
  );
}

// ─── Stat Card ────────────────────────────────────────────────────────────────
function StatCard({
  icon: Icon,
  label,
  value,
  color,
}: {
  icon: React.ElementType;
  label: string;
  value: string;
  color: string;
}) {
  return (
    <div className="glass rounded-xl p-4 flex items-center gap-4">
      <div className={cn("p-2.5 rounded-lg", color)}>
        <Icon size={18} />
      </div>
      <div>
        <p className="text-xs text-muted-foreground">{label}</p>
        <p className="text-lg font-semibold">{value}</p>
      </div>
    </div>
  );
}

// ─── Add Download Modal ───────────────────────────────────────────────────────
function AddDownloadModal({
  open,
  onClose,
  onStart,
  isLoading,
  error,
}: {
  open: boolean;
  onClose: () => void;
  onStart: (url: string) => void;
  isLoading?: boolean;
  error?: string | null;
}) {
  const [url, setUrl] = useState("");

  // Clear input when modal opens
  useEffect(() => {
    if (open) setUrl("");
  }, [open]);

  if (!open) return null;
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm">
      <div className="glass rounded-2xl p-6 w-full max-w-md shadow-2xl border border-border/60">
        <h2 className="text-lg font-semibold mb-4 gradient-text">New Download</h2>
        <label className="block text-sm text-muted-foreground mb-1.5">URL</label>
        <input
          id="new-download-url"
          type="text"
          value={url}
          autoFocus
          disabled={isLoading}
          onChange={(e) => setUrl(e.target.value)}
          onKeyDown={(e) => { if (e.key === "Enter" && url.trim()) onStart(url.trim()); }}
          placeholder="https://example.com/file.zip"
          className="w-full bg-secondary/50 border border-border rounded-lg px-3 py-2 text-sm outline-none focus:border-primary/60 focus:ring-1 focus:ring-primary/30 transition-all duration-200 mb-2 disabled:opacity-50"
        />
        {error && (
          <p className="text-xs text-red-400 mb-3 px-1 border border-red-500/30 bg-red-500/10 rounded-lg py-2">
            ⚠ {error}
          </p>
        )}
        <div className="flex gap-2 justify-end mt-2">
          <Button variant="ghost" size="sm" onClick={onClose} disabled={isLoading}>Cancel</Button>
          <Button
            id="btn-start-download"
            size="sm"
            disabled={!url.trim() || isLoading}
            onClick={() => { if (url.trim()) onStart(url.trim()); }}
          >
            {isLoading ? (
              <span className="flex items-center gap-2">
                <span className="w-3 h-3 border-2 border-primary-foreground/40 border-t-primary-foreground rounded-full animate-spin" />
                Starting…
              </span>
            ) : (
              <>
                <DownloadIcon size={14} />
                Start Download
              </>
            )}
          </Button>
        </div>
      </div>
    </div>
  );
}

// ─── Main App ─────────────────────────────────────────────────────────────────
export default function App() {
  const [backendReady, setBackendReady] = useState(false);
  const [activeNav, setActiveNav] = useState("downloads");
  const [showAddModal, setShowAddModal] = useState(false);
  const [downloads, setDownloads] = useState<Download[]>([]);
  const [modalLoading, setModalLoading] = useState(false);
  const [modalError, setModalError] = useState<string | null>(null);

  const fetchDownloads = async () => {
    try {
      const activeDownloads = await invoke<Download[]>("get_downloads");
      setDownloads(activeDownloads);
    } catch (error) {
      console.error("Failed to fetch downloads:", error);
    }
  };

  useEffect(() => {
    // 1. Initial Checks
    invoke<string>("ping")
      .then((res) => setBackendReady(res === "pong"))
      .catch(console.error);

    // Fetch initial list
    fetchDownloads();
  }, []);

  useEffect(() => {
    // 2. Event Listener - Performance Tuned React Cycle
    let animationFrameId: number;
    let isDirty = false;
    
    // Memory structs to accumulate fast signals
    const chunkData: Record<string, Record<string, number>> = {};
    const speedData: Record<string, Record<string, number>> = {};

    let unlistenFn: (() => void) | null = null;

    const attachIPC = async () => {
      unlistenFn = await listen<ProgressPayload>("download-progress", (event) => {
        const { download_id, chunk_id, downloaded_bytes, speed_bytes_per_sec } = event.payload;
        
        if (!chunkData[download_id]) {
          chunkData[download_id] = {};
          speedData[download_id] = {};
        }
        
        // Update chunk snapshot
        chunkData[download_id][chunk_id] = downloaded_bytes;
        speedData[download_id][chunk_id] = speed_bytes_per_sec;
        isDirty = true;
      });
    };

    attachIPC();

    // 60FPS UI reconciliation to prevent React stuttering under huge workload
    const updateUI = () => {
      if (isDirty) {
        setDownloads((prev) => 
          prev.map((d) => {
            if (chunkData[d.id]) {
              const liveDownloaded = Object.values(chunkData[d.id]).reduce((acc, bytes) => acc + bytes, 0);
              const liveSpeed = Object.values(speedData[d.id]).reduce((acc, spd) => acc + spd, 0);
              
              let status = d.status;
              if (liveDownloaded > 0 && liveDownloaded >= d.total_bytes && d.status !== "completed") {
                status = "completed";
              } else if (d.status === "pending" && liveDownloaded > 0) {
                status = "downloading";
              }

              return {
                ...d,
                status,
                _live_downloaded: liveDownloaded,
                _live_speed: liveSpeed,
              };
            }
            return d;
          })
        );
        isDirty = false;
      }
      animationFrameId = requestAnimationFrame(updateUI);
    };

    animationFrameId = requestAnimationFrame(updateUI);

    return () => {
      cancelAnimationFrame(animationFrameId);
      if (unlistenFn) unlistenFn();
    };
  }, []);

  // 3. Action Handlers
  const handleStartDownload = async (url: string) => {
    if (!url.trim()) return;
    setModalLoading(true);
    setModalError(null);

    let saveDir: string;
    try {
      saveDir = await downloadDir();
    } catch {
      try {
        saveDir = await invoke<string>("get_download_dir");
      } catch {
        saveDir = "C:\\Users\\Public\\Downloads";
      }
    }

    const guessedFilename = url.split("/").filter(Boolean).pop() ?? "download";
    const optimisticId = `optimistic-${Date.now()}`;
    const optimisticEntry: Download = {
      id: optimisticId,
      url,
      filename: guessedFilename,
      save_path: saveDir,
      total_bytes: 0,
      downloaded_bytes: 0,
      status: "pending",
      created_at: new Date().toISOString(),
      _optimistic: true,
    };

    setDownloads((prev) => [optimisticEntry, ...prev]);
    setShowAddModal(false);

    try {
      // Invoke returns the REAL download_id immediately
      const realId = await invoke<string>("start_download", { url, saveDir });
      
      // Update the optimistic entry with the real ID so event listener can bind to it
      setDownloads((prev) => 
        prev.map((d) => d.id === optimisticId ? { ...d, id: realId, _optimistic: false } : d)
      );
    } catch (e: unknown) {
      const errMsg = e instanceof Error ? e.message : String(e);
      console.error("[Tharsis] start_download failed:", errMsg);
      setModalError(errMsg);
      setShowAddModal(true);
      setDownloads((prev) => prev.filter((d) => d.id !== optimisticId));
    } finally {
      setModalLoading(false);
    }
  };

  // 4. Formatters
  const formatBytes = (b: number) => {
    if (b >= 1_073_741_824) return `${(b / 1_073_741_824).toFixed(2)} GB`;
    if (b >= 1_048_576) return `${(b / 1_048_576).toFixed(2)} MB`;
    return `${(b / 1024).toFixed(2)} KB`;
  };

  const formatSpeed = (bytesPerSec: number) => {
    if (!bytesPerSec || bytesPerSec === 0) return "-- KB/s";
    if (bytesPerSec >= 1_048_576) return `${(bytesPerSec / 1_048_576).toFixed(1)} MB/s`;
    return `${(bytesPerSec / 1024).toFixed(1)} KB/s`;
  };

  const formatETA = (seconds: number) => {
    if (!seconds || seconds <= 0 || seconds === Infinity) return "--";
    if (seconds >= 3600) {
      const h = Math.floor(seconds / 3600);
      const m = Math.floor((seconds % 3600) / 60);
      return `${h}h ${m}m`;
    }
    if (seconds >= 60) {
      const m = Math.floor(seconds / 60);
      const s = Math.floor(seconds % 60);
      return `${m}m ${s}s`;
    }
    return `${Math.ceil(seconds)}s`;
  };

  const calculateProgress = (item: Download) => {
    const downloaded = item._live_downloaded ?? item.downloaded_bytes;
    return item.total_bytes > 0 ? (downloaded / item.total_bytes) * 100 : 0;
  };

  const getDownloadedBytes = (item: Download) => {
    return item._live_downloaded ?? item.downloaded_bytes;
  };

  // Aggregation for Stats Header
  const activeCount = downloads.filter((d) => d.status === "downloading").length;
  const totalSavedBytes = downloads.reduce((acc, d) => acc + (d.status === "completed" ? d.total_bytes : 0), 0);
  const globalSpeed = downloads.reduce((acc, d) => acc + (d._live_speed || 0), 0);

  // Calculate Global ETA
  const remainingBytes = downloads
    .filter((d) => d.status === "downloading")
    .reduce((acc, d) => acc + Math.max(0, d.total_bytes - (d._live_downloaded ?? d.downloaded_bytes)), 0);
  const globalETA = globalSpeed > 0 ? remainingBytes / globalSpeed : 0;

  const handleDeleteDownload = async (id: string) => {
    // Optimistic removal
    setDownloads((prev) => prev.filter((d) => d.id !== id));
    try {
      await invoke("delete_download", { id });
    } catch (e) {
      console.error("[Tharsis] delete_download failed:", e);
      // Re-fetch to restore consistency
      fetchDownloads();
    }
  };

  return (
    <div className="flex h-screen overflow-hidden select-none">
      {/* ── Sidebar ─────────────────────────────────────── */}
      <aside className="w-56 flex flex-col glass border-r border-border/50 p-4 gap-1 z-10">
        <div className="flex items-center gap-2.5 px-2 py-3 mb-3">
          <div className="w-8 h-8 rounded-lg bg-primary/20 border border-primary/40 flex items-center justify-center">
            <Zap size={16} className="text-primary" />
          </div>
          <span className="font-bold text-base gradient-text tracking-tight">Tharsis</span>
        </div>

        <NavItem icon={DownloadIcon} label="Downloads" active={activeNav === "downloads"} onClick={() => setActiveNav("downloads")} />
        <NavItem icon={Activity} label="Activity" active={activeNav === "activity"} onClick={() => setActiveNav("activity")} />
        <NavItem icon={FolderOpen} label="Files" active={activeNav === "files"} onClick={() => setActiveNav("files")} />
        <NavItem icon={Settings} label="Settings" active={activeNav === "settings"} onClick={() => setActiveNav("settings")} />

        <div className="mt-auto pt-3 border-t border-border/50">
          <div className={cn(
            "flex items-center gap-2 px-3 py-2 rounded-lg text-xs",
            backendReady ? "text-green-400" : "text-muted-foreground"
          )}>
            <div className={cn(
              "w-1.5 h-1.5 rounded-full",
              backendReady ? "bg-green-400 animate-pulse" : "bg-muted-foreground"
            )} />
            {backendReady ? "Engine Ready" : "Connecting…"}
          </div>
        </div>
      </aside>

      {/* ── Main Content ───────────────────────────────── */}
      <main className="flex-1 flex flex-col overflow-hidden">
        {/* Header */}
        <header className="flex items-center justify-between px-6 py-4 border-b border-border/50 glass">
          <div>
            <h1 className="text-xl font-bold">Downloads</h1>
            <p className="text-xs text-muted-foreground">{downloads.length} items</p>
          </div>
          <Button id="btn-add-download" size="sm" onClick={() => setShowAddModal(true)}>
            <Plus size={16} />
            Add Download
          </Button>
        </header>

        {/* Stats Row */}
        <div className="grid grid-cols-4 gap-4 px-6 py-4">
          <StatCard icon={DownloadIcon} label="Active" value={activeCount.toString()} color="bg-blue-500/20 text-blue-400" />
          <StatCard icon={HardDrive} label="Total Saved" value={formatBytes(totalSavedBytes)} color="bg-purple-500/20 text-purple-400" />
          <StatCard icon={Zap} label="Global Speed" value={formatSpeed(globalSpeed)} color="bg-primary/20 text-primary" />
          <StatCard icon={Clock} label="ETA" value={formatETA(globalETA)} color="bg-orange-500/20 text-orange-400" />
        </div>

        {/* Download List */}
        <section className="flex-1 overflow-y-auto px-6 pb-6 space-y-3 relative z-0">
          {downloads.length === 0 ? (
            <div className="h-full flex flex-col items-center justify-center text-muted-foreground opacity-50">
              <DownloadIcon size={48} className="mb-4" />
              <p>No downloads yet.</p>
            </div>
          ) : (
            downloads.map((item) => (
              <div key={item.id} className="glass-hover rounded-xl p-4 group border border-transparent hover:border-white/5 transition-all">
                <div className="flex items-start gap-4">
                  {/* File icon */}
                  <div className="w-10 h-10 rounded-lg bg-secondary/80 flex items-center justify-center shrink-0 mt-0.5 border border-border/30 shadow-inner">
                    <Globe size={18} className="text-primary/70" />
                  </div>

                  <div className="flex-1 min-w-0">
                    <div className="flex items-center justify-between gap-2 mb-1.5">
                      <p className="text-sm font-semibold truncate text-foreground/90">{item.filename}</p>
                      <StatusBadge status={item.status} />
                    </div>

                    <p className="text-[11px] text-muted-foreground truncate mb-2 lg:w-3/4">{item.url}</p>

                    {item.status !== "completed" && (
                      <ProgressBar value={calculateProgress(item)} />
                    )}

                    <div className="flex items-center justify-between mt-2">
                      <span className="text-[11px] text-muted-foreground font-medium tracking-wide">
                        {formatBytes(getDownloadedBytes(item))} / {formatBytes(item.total_bytes)}
                      </span>
                      {item.status === "downloading" && (
                        <div className="flex items-center gap-3">
                          <span className="text-[11px] bg-secondary/60 px-2 py-0.5 rounded text-foreground/80 font-mono border border-white/5">
                            {formatSpeed(item._live_speed || 0)}
                          </span>
                          <span className="text-[11px] text-primary font-bold">
                            {calculateProgress(item).toFixed(1)}%
                          </span>
                        </div>
                      )}
                    </div>
                  </div>

                  {/* Actions */}
                  <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity duration-200">
                    {item.status === "downloading" && (
                      <Button variant="ghost" size="icon" className="h-8 w-8 hover:bg-secondary" title="Pause">
                        <Pause size={14} />
                      </Button>
                    )}
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-8 w-8 text-destructive/70 hover:text-destructive hover:bg-destructive/10"
                      title="Remove"
                      onClick={() => handleDeleteDownload(item.id)}
                    >
                      <Trash2 size={14} />
                    </Button>
                  </div>
                </div>
              </div>
            ))
          )}
        </section>
      </main>

      <AddDownloadModal 
        open={showAddModal} 
        onClose={() => { setShowAddModal(false); setModalError(null); }}
        onStart={handleStartDownload}
        isLoading={modalLoading}
        error={modalError}
      />
    </div>
  );
}
