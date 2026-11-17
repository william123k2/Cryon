import { useEffect, useState, useRef } from "react";
import {
  Home,
  Layers,
  Box,
  Download,
  Settings,
  ChevronRight,
  ArrowUpRight,
  Play,
  ShieldCheck,
  Zap,
  Plus,
  Pencil,
  Trash2,
  Check,
  Info,
  X,
  RefreshCw,
  Square,
  Terminal,
} from "lucide-react";
import { Crystal } from "./components/Crystal";
import { Field } from "./components/Field";
import { Transfer } from "./components/Transfer";
import { Preferences } from "./components/Preferences";
import { InstanceDialog } from "./components/InstanceDialog";
import {
  command,
  type Saved,
  type Snapshot,
  type Instance,
  type Version,
} from "./bridge";
type Page = "Início" | "Instâncias" | "Versões" | "Downloads" | "Configurações";
const navigation = [
  { name: "Início", icon: Home },
  { name: "Instâncias", icon: Layers },
  { name: "Versões", icon: Box },
  { name: "Downloads", icon: Download },
  { name: "Configurações", icon: Settings },
] as const;
export default function App() {
  const [page, setPage] = useState<Page>("Início"),
    [snapshot, setSnapshot] = useState<Snapshot | null>(null),
    [saved, setSaved] = useState<Saved | null>(null),
    [catalog, setCatalog] = useState<Version[]>([]),
    [catalogBusy, setCatalogBusy] = useState(false),
    [search, setSearch] = useState(""),
    [snapshots, setSnapshots] = useState(false),
    [notice, setNotice] = useState(""),
    [dialog, setDialog] = useState<Instance | null>(null),
    [remove, setRemove] = useState<Instance | null>(null),
    [loadError, setLoadError] = useState("");
  const configJson = useRef("");
  const mounted = useRef(true);
  async function refresh() {
    try {
      const s = await command<Snapshot>("snapshot");
      if (!mounted.current) return;
      setSnapshot(s);
      const json = JSON.stringify(s.saved);
      if (json !== configJson.current) {
        configJson.current = json;
        setSaved(s.saved);
      }
      setLoadError("");
    } catch (e) {
      if (mounted.current) setLoadError(String(e));
    }
  }
  useEffect(() => {
    mounted.current = true;
    void refresh();
    const t = setInterval(() => void refresh(), 1000);
    return () => {
      mounted.current = false;
      clearInterval(t);
    };
  }, []);
  async function loadVersions(force = false) {
    setCatalogBusy(true);
    try {
      setCatalog(await command<Version[]>("versions", { refresh: force }));
    } catch (e) {
      setNotice(String(e));
    } finally {
      setCatalogBusy(false);
    }
  }
  useEffect(() => {
    if (saved && !catalog.length && !catalogBusy) void loadVersions();
  }, [!!saved]);
  async function save(next: Saved) {
    const result = await command<Saved>("save_state", { saved: next });
    configJson.current = JSON.stringify(result);
    setSaved(result);
    await refresh();
  }
  const current = saved?.instances.find((i) => i.id === saved.selected);
  const busy = snapshot?.progress.status === "running";
  const running = !!snapshot?.game.running;
  const disabled = busy || running;
  async function start(launch: boolean, version?: string) {
    if (!saved || !current) return;
    try {
      if (version) {
        await save({
          ...saved,
          instances: saved.instances.map((i) =>
            i.id === current.id ? { ...i, version } : i,
          ),
        });
      }
      await command("start_operation", { instanceId: current.id, launch });
      await refresh();
      if (!launch) setPage("Downloads");
    } catch (e) {
      setNotice(String(e));
    }
  }
  async function chooseVersion(version: string) {
    if (!saved || !current) {
      setNotice("Crie uma instância primeiro.");
      setPage("Instâncias");
      return;
    }
    try {
      await save({
        ...saved,
        instances: saved.instances.map((i) =>
          i.id === current.id ? { ...i, version } : i,
        ),
      });
      setNotice("Versão selecionada: " + version);
    } catch (e) {
      setNotice(String(e));
    }
  }
  async function select(id: string) {
    if (!saved) return;
    try {
      await save({ ...saved, selected: id });
    } catch (e) {
      setNotice(String(e));
    }
  }
  function create() {
    if (!saved) return;
    const s = saved.settings;
    const id = crypto.randomUUID();
    setDialog({
      id,
      name: "Nova aventura",
      version: catalog.find((v) => v.type === "release")?.id || "1.21.4",
      directory: s.game_directory.replace(/[\\/]$/, "") + "\\" + id,
      min_ram: s.min_ram,
      max_ram: s.max_ram,
      width: s.width,
      height: s.height,
    });
  }
  async function storeInstance(instance: Instance) {
    if (!saved) return;
    await save({
      ...saved,
      selected: instance.id,
      instances: saved.instances.some((i) => i.id === instance.id)
        ? saved.instances.map((i) => (i.id === instance.id ? instance : i))
        : [...saved.instances, instance],
    });
    setNotice("Instância salva no computador.");
  }
  const name = saved?.profile.name || "Explorador";
  return (
    <div className="app">
      <aside className="sidebar">
        <a
          className="brand"
          href="#"
          onClick={(e) => {
            e.preventDefault();
            setPage("Início");
          }}
        >
          <Crystal />
          <div>
            <strong>
              CRYON<span>LAUNCHER</span>
            </strong>
            <small>Seu próximo universo.</small>
          </div>
        </a>
        <div className="nav-caption">ESPAÇO DE JOGO</div>
        <nav>
          {navigation.map(({ name: label, icon: Icon }) => (
            <button
              key={label}
              className={page === label ? "nav active" : "nav"}
              onClick={() => setPage(label)}
            >
              <Icon size={21} />
              {label}
              {page === label && <ChevronRight size={16} />}
            </button>
          ))}
        </nav>
        <div className="sidebar-bottom">
          <div className="build-label">
            <span className="dot" /> VANILLA · OFFLINE <span>0.2.0</span>
          </div>
          <div className="profile">
            <div className="avatar">{name.slice(0, 2).toUpperCase()}</div>
            <div>
              <strong>{name}</strong>
              <small>Perfil local</small>
            </div>
            <button
              className="icon-button"
              aria-label="Editar perfil"
              onClick={() => setPage("Configurações")}
            >
              <Settings size={17} />
            </button>
          </div>
        </div>
      </aside>
      <main>
        <header>
          <div className="breadcrumb">
            Cryon <ChevronRight size={13} />
            <span>{page}</span>
          </div>
          <span className="stage">VANILLA · WINDOWS X64</span>
        </header>
        {loadError && (
          <div role="alert" className="callout error">
            <Info size={18} />
            <span>{loadError}</span>
            <button onClick={() => void refresh()}>Tentar novamente</button>
          </div>
        )}
        {!saved || !snapshot ? (
          <div className="empty panel">
            <Crystal />
            <h2>
              {loadError
                ? "Não foi possível carregar o Cryon"
                : "Abrindo seu espaço…"}
            </h2>
            <p>Use o aplicativo desktop para instalar e jogar.</p>
          </div>
        ) : (
          <>
            {page === "Início" ? (
              <>
                <section className="hero">
                  <div className="hero-grid" />
                  <div className="hero-art" aria-hidden="true">
                    <div className="orbit one" />
                    <div className="orbit two" />
                    <Crystal />
                    <div className="pedestal" />
                  </div>
                  <div className="hero-copy">
                    <div className="eyebrow">
                      <span /> SEU MUNDO COMEÇA AQUI
                    </div>
                    <h1>
                      Grandes mundos.
                      <br />
                      <span>Novas histórias.</span>
                    </h1>
                    <p>
                      Um espaço para todas as suas aventuras.
                      <br />
                      Seu Minecraft, do seu jeito.
                    </p>
                    <div className="hero-features">
                      <span>
                        <Zap size={15} /> Java automático
                      </span>
                      <span>
                        <ShieldCheck size={15} /> Arquivos verificados
                      </span>
                    </div>
                  </div>
                  <div className="hero-footer">
                    <span>VANILLA EXPERIENCE</span>
                    <span>
                      Explore. Construa. Reinvente. <ArrowUpRight size={15} />
                    </span>
                  </div>
                </section>
                <section className="launch-panel">
                  <div className="launch-choice">
                    <span>JOGADOR</span>
                    <strong>
                      <div className="mini-avatar" />
                      {name}
                    </strong>
                    <small>Perfil offline</small>
                  </div>
                  <Field label="INSTÂNCIA">
                    <select
                      aria-label="Instância selecionada"
                      disabled={disabled}
                      value={current?.id || ""}
                      onChange={(e) => void select(e.target.value)}
                    >
                      {!saved.instances.length && (
                        <option value="">Crie uma instância</option>
                      )}
                      {saved.instances.map((i) => (
                        <option key={i.id} value={i.id}>
                          {i.name}
                        </option>
                      ))}
                    </select>
                  </Field>
                  <div className="launch-choice">
                    <span>VERSÃO</span>
                    <strong>
                      {current?.version || "Selecione uma versão"}
                    </strong>
                    <small>
                      {snapshot.installed.includes(current?.version || "")
                        ? "Instalada · será verificada ao jogar"
                        : "Instalação automática ao jogar"}
                    </small>
                  </div>
                  {running ? (
                    <button
                      className="play"
                      onClick={() =>
                        command("stop_game").catch((e) => setNotice(String(e)))
                      }
                    >
                      <Square size={20} /> ENCERRAR
                    </button>
                  ) : (
                    <button
                      className="play"
                      disabled={busy || !current?.version}
                      onClick={() => void start(true)}
                    >
                      <Play size={24} fill="currentColor" />
                      {busy ? "PREPARANDO" : "JOGAR"}
                    </button>
                  )}
                  <div className="launch-status">
                    <span className="dot" />
                    {running
                      ? `Minecraft ${snapshot.game.version} em execução · PID ${snapshot.game.pid}`
                      : busy
                        ? snapshot.progress.stage
                        : "Perfil local para jogar offline. Sem autenticação Microsoft ou acesso a servidores oficiais."}
                  </div>
                </section>
                {(busy ||
                  snapshot.progress.status === "error" ||
                  snapshot.progress.status === "cancelled") && (
                  <Transfer progress={snapshot.progress} notify={setNotice} />
                )}
                <div className="section-heading">
                  <h2>Seu ponto de partida</h2>
                  <span>INFORMAÇÕES DO SEU LAUNCHER</span>
                </div>
                <section className="cards">
                  {[
                    {
                      title: "Cada mundo, seu espaço",
                      text: `${saved.instances.length} instância(s) configurada(s). Mundos e ajustes separados.`,
                      page: "Instâncias" as Page,
                      Icon: Layers,
                      tag: "ORGANIZE",
                      art: "cubes",
                    },
                    {
                      title: "Versões oficiais",
                      text: `${snapshot.installed.length} versão(ões) instalada(s). Releases e snapshots do catálogo Mojang.`,
                      page: "Versões" as Page,
                      Icon: Box,
                      tag: "DESCUBRA",
                      art: "versions",
                    },
                    {
                      title: "No controle da aventura",
                      text: "Configure memória, resolução e Java. Seus ajustes ficam salvos no computador.",
                      page: "Configurações" as Page,
                      Icon: Settings,
                      tag: "PERSONALIZE",
                      art: "tuning",
                    },
                  ].map((c, i) => (
                    <button
                      className="info-card"
                      key={c.title}
                      onClick={() => setPage(c.page)}
                    >
                      <div className={"card-art " + c.art}>
                        <c.Icon size={40} />
                        <span>0{i + 1}</span>
                      </div>
                      <div className="card-content">
                        <span className="tag">{c.tag}</span>
                        <h3>{c.title}</h3>
                        <p>{c.text}</p>
                        <span className="card-link">
                          Abrir {c.page.toLowerCase()}{" "}
                          <ArrowUpRight size={16} />
                        </span>
                      </div>
                    </button>
                  ))}
                </section>
              </>
            ) : (
              <>
                <div className="page-heading">
                  <div>
                    <div className="eyebrow">SEU ESPAÇO CRYON</div>
                    <h1>{page}</h1>
                    <p>
                      {page === "Instâncias"
                        ? "Mundos separados. Preferências que acompanham você."
                        : page === "Versões"
                          ? "Catálogo oficial Mojang. Releases e snapshots Vanilla."
                          : page === "Downloads"
                            ? "Downloads reais, integridade e logs da sua sessão."
                            : "Seu perfil e suas preferências, salvos no computador."}
                    </p>
                  </div>
                  {page === "Instâncias" && (
                    <button
                      className="primary"
                      disabled={disabled}
                      onClick={create}
                    >
                      <Plus size={18} /> Nova instância
                    </button>
                  )}
                </div>
                {page === "Instâncias" && (
                  <>
                    <div className="callout">
                      <Info size={18} /> Excluir uma instância remove somente
                      seu cadastro. Mundos e arquivos permanecem na pasta.
                    </div>
                    <div className="instance-list">
                      {saved.instances.map((i) => (
                        <article
                          className={
                            "instance " +
                            (i.id === saved.selected ? "selected" : "")
                          }
                          key={i.id}
                        >
                          <div className="instance-icon">
                            <Box />
                          </div>
                          <div>
                            <h3>{i.name}</h3>
                            <p>
                              {i.version || "Sem versão"} · {i.min_ram}–
                              {i.max_ram} MB · {i.width}×{i.height}
                            </p>
                            <small>{i.directory}</small>
                          </div>
                          <div className="instance-actions">
                            <button
                              disabled={disabled}
                              onClick={() => void select(i.id)}
                            >
                              {i.id === saved.selected ? (
                                <Check size={16} />
                              ) : (
                                <Play size={16} />
                              )}{" "}
                              {i.id === saved.selected
                                ? "Selecionada"
                                : "Selecionar"}
                            </button>
                            <button
                              className="icon-button"
                              aria-label={`Editar ${i.name}`}
                              disabled={disabled}
                              onClick={() => setDialog({ ...i })}
                            >
                              <Pencil size={17} />
                            </button>
                            <button
                              className="icon-button danger"
                              aria-label={`Excluir ${i.name}`}
                              disabled={disabled}
                              onClick={() => setRemove(i)}
                            >
                              <Trash2 size={17} />
                            </button>
                          </div>
                        </article>
                      ))}
                      {!saved.instances.length && (
                        <div className="empty">
                          <Layers />
                          <h2>Sua próxima aventura começa aqui</h2>
                          <p>
                            Crie uma instância para escolher versão, pasta e
                            memória.
                          </p>
                        </div>
                      )}
                    </div>
                  </>
                )}
                {page === "Versões" && (
                  <>
                    <div className="catalog-toolbar">
                      <input
                        aria-label="Buscar versão"
                        placeholder="Buscar versão: 1.21.4, 1.16.5…"
                        value={search}
                        onChange={(e) => setSearch(e.target.value)}
                      />
                      <label className="toggle">
                        <input
                          type="checkbox"
                          checked={snapshots}
                          onChange={(e) => setSnapshots(e.target.checked)}
                        />{" "}
                        Incluir snapshots
                      </label>
                      <button
                        disabled={catalogBusy}
                        onClick={() => void loadVersions(true)}
                      >
                        <RefreshCw size={16} />
                        {catalogBusy ? "Consultando…" : "Atualizar catálogo"}
                      </button>
                    </div>
                    <div className="callout">
                      <Info size={18} /> As ações usam a instância selecionada:{" "}
                      <strong>{current?.name || "nenhuma"}</strong>. Versões
                      antigas e snapshots podem exigir drivers ou Java
                      específicos.
                    </div>
                    <div className="version-list panel">
                      {catalog
                        .filter(
                          (v) =>
                            (snapshots || v.type === "release") &&
                            v.id.toLowerCase().includes(search.toLowerCase()),
                        )
                        .slice(0, 200)
                        .map((v) => (
                          <div className="version-row" key={v.id}>
                            <Box size={21} />
                            <div>
                              <strong>{v.id}</strong>
                              <small>
                                {v.type} ·{" "}
                                {v.releaseTime
                                  ? new Date(v.releaseTime).toLocaleDateString(
                                      "pt-BR",
                                    )
                                  : ""}
                              </small>
                            </div>
                            {snapshot.installed.includes(v.id) && (
                              <span className="tag">INSTALADA</span>
                            )}
                            <div className="version-actions">
                              <button
                                disabled={disabled || !current}
                                onClick={() => void chooseVersion(v.id)}
                              >
                                Usar versão
                              </button>
                              <button
                                className="primary"
                                disabled={disabled || !current}
                                onClick={() => void start(false, v.id)}
                              >
                                <Download size={15} />
                                {snapshot.installed.includes(v.id)
                                  ? "Verificar"
                                  : "Instalar"}
                              </button>
                            </div>
                          </div>
                        ))}
                      {!catalog.length && (
                        <div className="empty">
                          <p>
                            {catalogBusy
                              ? "Consultando o manifesto oficial…"
                              : "O catálogo ainda não foi carregado. Clique em Atualizar catálogo."}
                          </p>
                        </div>
                      )}
                    </div>
                  </>
                )}
                {page === "Downloads" && (
                  <>
                    <Transfer progress={snapshot.progress} notify={setNotice} />
                    {snapshot.progress.status === "error" && current && (
                      <button
                        className="primary"
                        disabled={disabled}
                        onClick={() =>
                          void start(snapshot.progress.kind === "launch")
                        }
                      >
                        <RefreshCw size={17} /> Tentar novamente
                      </button>
                    )}
                    <section className="panel log-panel">
                      <div className="section-heading">
                        <h2>
                          <Terminal size={18} /> Logs da sessão
                        </h2>
                        <button
                          onClick={() =>
                            command("export_logs")
                              .then(
                                (p) => p && setNotice("Diagnóstico exportado."),
                              )
                              .catch((e) => setNotice(String(e)))
                          }
                        >
                          Exportar
                        </button>
                      </div>
                      <pre aria-label="Logs do launcher">
                        {snapshot.logs.length
                          ? snapshot.logs.join("\n")
                          : "Nenhuma operação nesta sessão. Logs completos são gravados na pasta privada do Cryon."}
                      </pre>
                    </section>
                  </>
                )}
                {page === "Configurações" && (
                  <Preferences
                    saved={saved}
                    snapshot={snapshot}
                    onSave={save}
                    notify={setNotice}
                  />
                )}
              </>
            )}
          </>
        )}
        <footer>
          <span>
            <Crystal small /> CRYON <span className="footer-separator">/</span>{" "}
            Feito para explorar.
          </span>
          <span>
            Projeto independente · Não afiliado à Mojang ou Microsoft.
          </span>
        </footer>
      </main>
      {notice && (
        <div role="status" className="toast">
          <Info size={20} />
          <span>{notice}</span>
          <button aria-label="Fechar aviso" onClick={() => setNotice("")}>
            <X size={18} />
          </button>
        </div>
      )}
      {dialog && (
        <InstanceDialog
          initial={dialog}
          versions={catalog}
          onSave={storeInstance}
          onClose={() => setDialog(null)}
          notify={setNotice}
        />
      )}
      {remove && saved && (
        <div className="modal-backdrop">
          <section
            className="modal"
            role="alertdialog"
            aria-modal="true"
            aria-labelledby="delete-title"
          >
            <h2 id="delete-title">Remover instância?</h2>
            <p>
              Remover “{remove.name}” do Cryon? Os arquivos do jogo e os mundos
              serão preservados.
            </p>
            <div className="dialog-actions">
              <button onClick={() => setRemove(null)}>Cancelar</button>
              <button
                className="danger"
                onClick={async () => {
                  try {
                    await save({
                      ...saved,
                      instances: saved.instances.filter(
                        (i) => i.id !== remove.id,
                      ),
                    });
                    setRemove(null);
                  } catch (e) {
                    setNotice(String(e));
                  }
                }}
              >
                <Trash2 size={16} /> Remover cadastro
              </button>
            </div>
          </section>
        </div>
      )}
    </div>
  );
}
