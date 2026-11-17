import { useState } from "react";
import { Check, Folder, Terminal, Download } from "lucide-react";
import { Field } from "./Field";
import { command, type Saved, type Snapshot } from "../bridge";
export function Preferences({
  saved,
  snapshot,
  onSave,
  notify,
}: {
  saved: Saved;
  snapshot: Snapshot;
  onSave: (s: Saved) => Promise<void>;
  notify: (s: string) => void;
}) {
  const [draft, setDraft] = useState(saved);
  const [saving, setSaving] = useState(false);
  const settings = draft.settings;
  const set = (k: keyof Saved["settings"], v: string | number | boolean) =>
    setDraft({ ...draft, settings: { ...settings, [k]: v } });
  return (
    <form
      onSubmit={async (e) => {
        e.preventDefault();
        setSaving(true);
        try {
          await onSave(draft);
          notify("Perfil e preferências salvos no computador.");
        } catch (e) {
          notify(String(e));
        } finally {
          setSaving(false);
        }
      }}
    >
      <div className="settings-grid">
        <section className="panel settings-section">
          <h2>Seu perfil local</h2>
          <Field
            label="Nome do jogador"
            hint="3–16 letras, números ou sublinhado. O UUID é calculado no backend."
          >
            <input
              required
              pattern="[A-Za-z0-9_]{3,16}"
              maxLength={16}
              value={draft.profile.name}
              onChange={(e) =>
                setDraft({
                  ...draft,
                  profile: { ...draft.profile, name: e.target.value },
                })
              }
            />
          </Field>
          <p>
            UUID salvo: <code>{saved.profile.uuid}</code>
          </p>
          <div className="callout">
            Perfil offline: não autentica em servidores que exigem conta
            Microsoft. OAuth não está implementado nesta edição.
          </div>
        </section>
        <section className="panel settings-section">
          <h2>Padrões de novas instâncias</h2>
          <Field label="Diretório do jogo">
            <div className="input-row">
              <input
                required
                value={settings.game_directory}
                onChange={(e) => set("game_directory", e.target.value)}
              />
              <button
                type="button"
                aria-label="Escolher pasta padrão"
                onClick={async () => {
                  try {
                    const p = await command<string | null>("pick_directory");
                    if (p) set("game_directory", p);
                  } catch (e) {
                    notify(String(e));
                  }
                }}
              >
                <Folder size={18} />
              </button>
            </div>
          </Field>
          <div className="two-fields">
            <Field label="RAM mínima (MB)">
              <input
                required
                type="number"
                min={512}
                max={65536}
                step={256}
                value={settings.min_ram}
                onChange={(e) => set("min_ram", +e.target.value)}
              />
            </Field>
            <Field label="RAM máxima (MB)">
              <input
                required
                type="number"
                min={settings.min_ram}
                max={65536}
                step={256}
                value={settings.max_ram}
                onChange={(e) => set("max_ram", +e.target.value)}
              />
            </Field>
          </div>
          <div className="two-fields">
            <Field label="Largura">
              <input
                required
                type="number"
                min={640}
                max={7680}
                value={settings.width}
                onChange={(e) => set("width", +e.target.value)}
              />
            </Field>
            <Field label="Altura">
              <input
                required
                type="number"
                min={480}
                max={4320}
                value={settings.height}
                onChange={(e) => set("height", +e.target.value)}
              />
            </Field>
          </div>
          <p>
            Instâncias existentes mantêm seus próprios limites de RAM e
            resolução.
          </p>
        </section>
        <section className="panel settings-section">
          <h2>Runtime Java</h2>
          <Field
            label="Executável Java"
            hint="Deixe vazio para usar Java privado do Cryon. O backend confere a versão antes de executar."
          >
            <div className="input-row">
              <input
                placeholder="Gerenciado automaticamente"
                value={settings.java_path}
                onChange={(e) => set("java_path", e.target.value)}
              />
              <button
                type="button"
                aria-label="Escolher Java"
                onClick={async () => {
                  try {
                    const p = await command<string | null>("pick_java");
                    if (p) set("java_path", p);
                  } catch (e) {
                    notify(String(e));
                  }
                }}
              >
                <Folder size={18} />
              </button>
            </div>
          </Field>
          <label className="toggle">
            <input
              type="checkbox"
              checked={settings.auto_java}
              onChange={(e) => set("auto_java", e.target.checked)}
            />{" "}
            Baixar Java compatível automaticamente
          </label>
          <p>
            Eclipse Temurin oficial, com SHA-256 verificado. Licenças do pacote
            preservadas. Nenhuma alteração no Java do sistema.
          </p>
        </section>
        <section className="panel settings-section">
          <h2>
            <Terminal size={20} /> Diagnóstico
          </h2>
          <dl>
            <dt>Cryon</dt>
            <dd>{snapshot.app_version}</dd>
            <dt>Ambiente</dt>
            <dd>
              {snapshot.runtime_os} {snapshot.runtime_arch}
            </dd>
            <dt>Pasta privada</dt>
            <dd>{snapshot.root}</dd>
            <dt>Jogo</dt>
            <dd>
              {snapshot.game.running
                ? `Executando · PID ${snapshot.game.pid}`
                : snapshot.game.exit_code !== null
                  ? `Última saída: ${snapshot.game.exit_code}`
                  : "Parado"}
            </dd>
          </dl>
          <button
            type="button"
            onClick={async () => {
              try {
                const p = await command<string | null>("export_logs");
                if (p) notify("Diagnóstico exportado: " + p);
              } catch (e) {
                notify(String(e));
              }
            }}
          >
            <Download size={16} /> Exportar diagnóstico
          </button>
        </section>
      </div>
      <div className="form-footer">
        <span>JSON versionado · backup automático</span>
        <button className="primary" type="submit" disabled={saving}>
          <Check size={17} />
          {saving ? "Salvando…" : "Salvar preferências"}
        </button>
      </div>
    </form>
  );
}
