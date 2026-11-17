import { Download, X, Check, AlertCircle } from "lucide-react";
import { bytes, command, type Progress } from "../bridge";
export function Transfer({
  progress: p,
  notify,
}: {
  progress: Progress;
  notify: (s: string) => void;
}) {
  const busy = p.status === "running";
  const percent = p.total_bytes
    ? Math.min(100, (p.processed_bytes / p.total_bytes) * 100)
    : 0;
  return (
    <section className="transfer panel" aria-live="polite">
      <div className="transfer-title">
        {busy ? (
          <Download size={22} />
        ) : p.status === "error" ? (
          <AlertCircle size={22} />
        ) : (
          <Check size={22} />
        )}
        <div>
          <strong>{p.stage || "Nenhum download em andamento"}</strong>
          <small>
            {p.error ||
              p.current ||
              "Downloads oficiais com verificação de integridade."}
          </small>
        </div>
        {busy && (
          <button
            onClick={() =>
              command("cancel_download").catch((e) => notify(String(e)))
            }
          >
            <X size={16} /> Cancelar
          </button>
        )}
      </div>
      <progress max={100} value={percent} aria-label="Progresso da operação" />
      <div className="transfer-meta">
        <span>
          {bytes(p.processed_bytes)} / {bytes(p.total_bytes)} processados ·{" "}
          {p.files_done}/{p.files_total} arquivos
        </span>
        <span>
          {percent.toFixed(1)}% · {bytes(p.speed)}/s
        </span>
      </div>
      <small>
        Recebidos pela rede: {bytes(p.downloaded_bytes)} · Tentativas
        adicionais: {p.retries}. Arquivos locais são verificados antes de
        reutilizados.
      </small>
    </section>
  );
}
