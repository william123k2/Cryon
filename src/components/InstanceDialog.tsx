import { useState } from "react";
import { X, Folder, Check } from "lucide-react";
import { Field } from "./Field";
import { command, type Instance, type Version } from "../bridge";
export function InstanceDialog({
  initial,
  versions,
  onSave,
  onClose,
  notify,
}: {
  initial: Instance;
  versions: Version[];
  onSave: (i: Instance) => Promise<void>;
  onClose: () => void;
  notify: (s: string) => void;
}) {
  const [draft, setDraft] = useState(initial);
  const [saving, setSaving] = useState(false);
  const set = (k: keyof Instance, v: string | number) =>
    setDraft({ ...draft, [k]: v });
  return (
    <div className="modal-backdrop">
      <section
        className="modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="instance-title"
      >
        <button
          className="modal-close icon-button"
          aria-label="Fechar"
          onClick={onClose}
        >
          <X />
        </button>
        <h2 id="instance-title">Configurar instância</h2>
        <form
          onSubmit={async (e) => {
            e.preventDefault();
            setSaving(true);
            try {
              await onSave({ ...draft, name: draft.name.trim() });
              onClose();
            } catch (e) {
              notify(String(e));
            } finally {
              setSaving(false);
            }
          }}
        >
          <Field label="Nome">
            <input
              autoFocus
              required
              maxLength={80}
              value={draft.name}
              onChange={(e) => set("name", e.target.value)}
            />
          </Field>
          <Field label="Versão Minecraft">
            <input
              required
              list="version-options"
              value={draft.version}
              onChange={(e) => set("version", e.target.value)}
              placeholder="1.21.4"
            />
            <datalist id="version-options">
              {versions.map((v) => (
                <option key={v.id} value={v.id} />
              ))}
            </datalist>
          </Field>
          <Field
            label="Pasta do jogo"
            hint="Mundos, mods e configurações ficam nesta pasta. A exclusão da instância no Cryon preserva seus arquivos."
          >
            <div className="input-row">
              <input
                required
                value={draft.directory}
                onChange={(e) => set("directory", e.target.value)}
              />
              <button
                type="button"
                aria-label="Escolher pasta da instância"
                onClick={async () => {
                  try {
                    const d = await command<string | null>("pick_directory");
                    if (d) set("directory", d);
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
                type="number"
                required
                min={512}
                max={65536}
                step={256}
                value={draft.min_ram}
                onChange={(e) => set("min_ram", +e.target.value)}
              />
            </Field>
            <Field label="RAM máxima (MB)">
              <input
                type="number"
                required
                min={draft.min_ram}
                max={65536}
                step={256}
                value={draft.max_ram}
                onChange={(e) => set("max_ram", +e.target.value)}
              />
            </Field>
          </div>
          <div className="two-fields">
            <Field label="Largura">
              <input
                type="number"
                required
                min={640}
                max={7680}
                value={draft.width}
                onChange={(e) => set("width", +e.target.value)}
              />
            </Field>
            <Field label="Altura">
              <input
                type="number"
                required
                min={480}
                max={4320}
                value={draft.height}
                onChange={(e) => set("height", +e.target.value)}
              />
            </Field>
          </div>
          <button className="primary" disabled={saving} type="submit">
            <Check size={17} />
            {saving ? "Salvando…" : "Salvar instância"}
          </button>
        </form>
      </section>
    </div>
  );
}
