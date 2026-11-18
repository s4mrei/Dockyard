import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useEffect, useRef, useState, type CSSProperties, type FormEvent } from 'react';
import ActionMenu from './ActionMenu';
import ConfirmDialog from './ConfirmDialog';
import AppearanceSettings from './AppearanceSettings';
import { defaultAppearance, type Appearance, type ItemKind } from './accents';
import { Card, type Application } from './cards';
import dockyardIcon from '../src-tauri/icons/icon.png';

interface WorkspaceItem {
  id: number;
  kind: ItemKind;
  target: string;
  name: string;
}

interface Workspace {
  id: number;
  name: string;
  items: WorkspaceItem[];
}

const filters = [
  ['all', 'Всё'], ['application', 'Приложения'], ['file', 'Файлы'], ['folder', 'Папки'],
] as const;

const collator = new Intl.Collator(undefined, { sensitivity: 'base' });

export default function App() {
  const [applications, setApplications] = useState<Application[] | null>(null);
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
  const [activeId, setActiveId] = useState<number | null>(null);
  const [destinationId, setDestinationId] = useState<number | null>(null);
  const [edit, setEdit] = useState<{ id: number | null; name: string } | null>(null);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [query, setQuery] = useState('');
  const [loadError, setLoadError] = useState('');
  const [storageError, setStorageError] = useState('');
  const [error, setError] = useState('');
  const [status, setStatus] = useState('');
  const [busy, setBusy] = useState(false);
  const [filter, setFilter] = useState<(typeof filters)[number][0]>('all');
  const [appearance, setAppearance] = useState<Appearance | null>(null);
  const [appearanceError, setAppearanceError] = useState('');
  const [draft, setDraft] = useState<Appearance | null>(null);
  const [settingsReset, setSettingsReset] = useState(0);
  const [pendingNavigation, setPendingNavigation] = useState<(() => void) | null>(null);
  const nextFocus = useRef<number | 'heading' | null>(null);
  const tabs = useRef<HTMLElement>(null);
  const heading = useRef<HTMLHeadingElement>(null);

  async function refreshWorkspaces() {
    setWorkspaces(await invoke<Workspace[]>('list_workspaces'));
  }

  useEffect(() => {
    let disposed = false;
    let generation = 0;
    const refreshApplications = async () => {
      const request = ++generation;
      try {
        const apps = await invoke<Application[]>('list_applications');
        if (!disposed && request === generation) {
          setApplications(apps.sort((a, b) => collator.compare(a.name, b.name)));
          setLoadError('');
        }
      } catch {
        if (!disposed && request === generation) setLoadError('Не удалось загрузить библиотеку приложений.');
      }
    };
    const subscription = listen('application-icons-changed', refreshApplications);
    void subscription.catch(() => {});
    void refreshApplications();
    window.addEventListener('focus', refreshApplications);
    refreshWorkspaces().catch(() => setStorageError('Не удалось загрузить рабочие пространства.'));
    invoke<Appearance>('load_appearance').then(setAppearance).catch(() => {
      setAppearanceError('Не удалось загрузить настройки оформления.');
    });
    return () => {
      disposed = true;
      window.removeEventListener('focus', refreshApplications);
      void subscription.then((unlisten) => unlisten()).catch(() => {});
    };
  }, []);

  const active = workspaces.find((tab) => tab.id === activeId);
  const destination = workspaces.find((tab) => tab.id === destinationId) ?? workspaces[0];
  const savedApps = new Set(destination?.items.filter((item) => item.kind === 'application').map((item) => item.target));
  const search = query.trim().toLocaleLowerCase();
  const matches = applications?.filter((app) => app.name.toLocaleLowerCase().includes(search));

  const settings = appearance ?? defaultAppearance;
  const shownSettings = draft ?? settings;
  const items = active?.items.filter((item) => filter === 'all' || filter === item.kind) ?? [];
  const applicationsById = new Map(applications?.map((app) => [app.id, app]));

  useEffect(() => {
    const tabBar = tabs.current;
    if (!tabBar) return;
    const revealActive = () => {
      tabBar.querySelector('[aria-current="page"]')?.scrollIntoView({ block: 'nearest', inline: 'nearest' });
    };
    revealActive();
    const observer = new ResizeObserver(revealActive);
    observer.observe(tabBar);
    return () => observer.disconnect();
  }, [activeId, !!draft]);

  useEffect(() => {
    if (busy || nextFocus.current === null) return;
    const node = typeof nextFocus.current === 'number'
      ? document.querySelector<HTMLElement>(`[data-item-id="${nextFocus.current}"] .card-main`)
      : null;
    (node ?? heading.current)?.focus();
    nextFocus.current = null;
  }, [busy, workspaces]);

  function appearanceForm() {
    return document.forms.namedItem('appearance-form');
  }

  function navigate(next: () => void) {
    if (draft && (draft !== settings || appearanceForm()?.querySelector(':invalid'))) {
      setError('');
      setPendingNavigation(() => next);
    } else {
      next();
    }
  }

  function selectTab(id: number | null) {
    setDraft(null);
    setFilter('all');
    setActiveId(id);
    if (id !== null) setDestinationId(id);
    setEdit(null);
    setConfirmDelete(false);
    setError('');
    setStatus('');
  }

  async function run(action: () => Promise<void>, fallback: string) {
    setError('');
    setStatus('');
    setBusy(true);
    try {
      await action();
    } catch (cause) {
      setError(typeof cause === 'string' ? cause : fallback);
    } finally {
      setBusy(false);
    }
  }

  function saveWorkspace(event: FormEvent) {
    event.preventDefault();
    if (!edit) return;
    void run(async () => {
      const id = await invoke<number>('save_workspace', edit);
      await refreshWorkspaces();
      selectTab(id);
    }, 'Не удалось сохранить вкладку.');
  }

  function deleteWorkspace() {
    if (!active) return;
    void run(async () => {
      await invoke('delete_workspace', { id: active.id });
      await refreshWorkspaces();
      selectTab(null);
    }, 'Не удалось удалить вкладку.');
  }

  function launchApplication(id: string) {
    return run(() => invoke('launch_application', { id }), 'Не удалось запустить приложение.');
  }

  function addApplication(app: Application) {
    if (!destination) return;
    void run(async () => {
      await invoke('add_workspace_application', { workspaceId: destination.id, desktopId: app.id });
      await refreshWorkspaces();
      setStatus(`«${app.name}» добавлено во вкладку «${destination.name}».`);
    }, 'Не удалось добавить приложение.');
  }

  function addLocalItem(folder: boolean) {
    if (!active) return;
    void run(async () => {
      await invoke('add_workspace_local_item', { workspaceId: active.id, folder });
      await refreshWorkspaces();
    }, 'Не удалось добавить ссылку.');
  }

  function removeItem(id: number) {
    return run(async () => {
      await invoke('remove_workspace_item', { id });
      const index = items.findIndex((item) => item.id === id);
      nextFocus.current = (items[index + 1] ?? items[index - 1])?.id ?? 'heading';
      await refreshWorkspaces();
    }, 'Не удалось убрать ссылку.');
  }

  function openItem(item: WorkspaceItem) {
    return item.kind === 'application'
      ? launchApplication(item.target)
      : run(() => invoke('open_workspace_local_item', { id: item.id }), 'Не удалось открыть ссылку.');
  }

  function openSettings() {
    setEdit(null);
    setConfirmDelete(false);
    setError('');
    setStatus('');
    setDraft(settings);
  }

  function saveAppearance(afterSave?: () => void) {
    if (!draft || appearanceForm()?.querySelector(':invalid')) return;
    void run(async () => {
      await invoke('save_appearance', { settings: draft });
      setAppearance(draft);
      if (afterSave) afterSave();
      else setStatus('Настройки сохранены.');
    }, 'Не удалось сохранить оформление.');
  }

  return (
    <main className="shell" style={{ '--glow-color': shownSettings.glowColor } as CSSProperties} data-glow={shownSettings.glow}>
      <header className="topbar">
        <div className="brand">
          <img src={dockyardIcon} alt="" />
          <h1>Dockyard</h1>
        </div>
        <nav ref={tabs} className="tabs" aria-label="Рабочие пространства">
          <button aria-current={!active && !draft ? "page" : undefined} disabled={busy} onClick={() => navigate(() => selectTab(null))}>Все приложения</button>
          {workspaces.map((tab) => (
            <button key={tab.id} title={tab.name} aria-current={activeId === tab.id && !draft ? "page" : undefined} disabled={busy}
              onClick={() => navigate(() => selectTab(tab.id))}>{tab.name}</button>
          ))}
          <button className="new-tab" aria-label="Новая вкладка" title="Новая вкладка" disabled={busy || !!storageError} onClick={() => navigate(() => {
            selectTab(activeId);
            setEdit({ id: null, name: '' });
          })}>+</button>
        </nav>
      </header>
      <div className="body-layout">
        <aside className="sidebar">
          {active && !draft && <nav className="filters" aria-label="Фильтры рабочего пространства">
            {filters.map(([id, label]) => <button key={id} aria-pressed={filter === id} onClick={() => setFilter(id)}>{label}</button>)}
          </nav>}
          <button className="settings-button" aria-pressed={!!draft} disabled={busy || !appearance}
            onClick={() => { if (!draft) openSettings(); }}>
            <span aria-hidden="true">⚙</span> Настройки
          </button>
        </aside>
        <section key={draft ? "appearance" : activeId ?? "library"} className="content" aria-label={draft ? 'Оформление' : active?.name ?? 'Библиотека приложений'}>
          <div className="section-heading">
            <h2 ref={heading} tabIndex={-1} className="section-title" title={active?.name}>
              {draft ? 'Оформление' : active?.name ?? 'Библиотека приложений'}
            </h2>
            {active && !draft && <div className="toolbar workspace-actions">
              <button disabled={busy} onClick={() => addLocalItem(false)}>Добавить файл</button>
              <button disabled={busy} onClick={() => addLocalItem(true)}>Добавить папку</button>
              <ActionMenu label={`Действия с пространством «${active.name}»`} className="workspace-menu" disabled={busy}
                actions={[
                  { label: 'Переименовать', run: () => setEdit({ id: active.id, name: active.name }) },
                  { label: 'Удалить пространство…', danger: true, run: () => { setError(''); setEdit(null); setConfirmDelete(true); } },
                ]} />
            </div>}
          </div>
          {storageError && <p className="notice" role="alert">{storageError}</p>}
          {appearanceError && <p className="notice" role="alert">{appearanceError}</p>}
          {error && <p className="notice" role="alert">{error}</p>}
          {status && <p className="notice" role="status">{status}</p>}

          {edit && <form className="editor" onSubmit={saveWorkspace} onKeyDown={(event) => {
            if (event.key === 'Escape' && !busy) { event.preventDefault(); setEdit(null); }
          }}>
            <label htmlFor="tab-name">Название вкладки</label>
            <div className="toolbar">
              <input id="tab-name" value={edit.name} maxLength={80} autoFocus required onFocus={(event) => event.target.select()}
                onChange={(event) => setEdit({ ...edit, name: event.target.value })} />
              <button className="primary" disabled={busy || !edit.name.trim()}>Сохранить</button>
              <button type="button" disabled={busy} onClick={() => setEdit(null)}>Отмена</button>
            </div>
          </form>}

          {draft ? (
            <AppearanceSettings key={settingsReset} draft={draft} change={setDraft} applications={applications ?? []}
              save={() => saveAppearance()} cancel={() => { setDraft(settings); setSettingsReset((value) => value + 1); }}
              busy={busy} dirty={draft !== settings} />
          ) : active ? (
            <>
              {active.items.length === 0 ? <div className="empty-state">
                <h3>Вкладка пока пуста</h3>
                <p>Добавьте файл или папку здесь, либо выберите приложение в библиотеке.</p>
                <button disabled={busy} onClick={() => selectTab(null)}>Перейти в библиотеку</button>
              </div> : items.length === 0 ? <p className="muted">Нет элементов этого типа.</p> : (
                <ul className="card-grid">
                  {items.map((item) => {
                    const app = item.kind === 'application' ? applicationsById.get(item.target) : undefined;
                    const name = app?.name ?? item.name;
                    return <li key={item.id} data-item-id={item.id}>
                      <Card name={name} kind={item.kind} appId={item.kind === 'application' ? item.target : undefined}
                        icon={app?.icon} settings={settings} label="Открыть" action={() => { void openItem(item); }} disabled={busy}
                        extra={<ActionMenu label={`Действия для «${name}»`} disabled={busy} actions={[
                          { label: 'Открыть', run: () => { void openItem(item); } },
                          { label: 'Убрать ссылку', danger: true, run: () => { void removeItem(item.id); } },
                        ]} />} />
                    </li>;
                  })}
                </ul>
              )}
            </>
          ) : (
            <>
              <div className="library-controls">
                <label htmlFor="app-search">Поиск по названию
                  <input id="app-search" type="search" placeholder="Найти приложение" value={query} onChange={(event) => setQuery(event.target.value)} />
                </label>
                {destination && <label htmlFor="destination">Добавлять во вкладку
                  <select id="destination" value={destination.id} disabled={busy}
                    onChange={(event) => setDestinationId(Number(event.target.value))}>
                    {workspaces.map((tab) => <option key={tab.id} value={tab.id}>{tab.name}</option>)}
                  </select>
                </label>}
              </div>
              {loadError && <p role="alert" className="notice">{loadError}</p>}
              {!applications && !loadError && <p role="status" className="muted">Загружаем приложения…</p>}
              {matches?.length === 0 && <p role="status" className="muted">{search ? 'Приложения не найдены.' : 'Библиотека пуста.'}</p>}
              {!!matches?.length && <ul className="card-grid">
                {matches.map((app) => {
                  const saved = savedApps.has(app.id);
                  return <li key={app.id}>
                    <Card name={app.name} kind="application" appId={app.id} icon={app.icon} settings={settings}
                      label="Запустить" action={() => { void launchApplication(app.id); }} disabled={busy}
                      extra={destination && <button className="card-extra" disabled={busy || saved}
                        title={saved ? 'Добавлено' : `Добавить в «${destination.name}»`}
                        aria-label={saved ? 'Уже добавлено' : `Добавить ${app.name} во вкладку ${destination.name}`}
                        onClick={() => addApplication(app)}>{saved ? '✓' : '+'}</button>} />
                  </li>;
                })}
              </ul>}
            </>
          )}
        </section>
      </div>
      {confirmDelete && active && <ConfirmDialog title="Удалить пространство?" cancel={() => { if (!busy) setConfirmDelete(false); }}
        actions={<button className="danger" disabled={busy} onClick={deleteWorkspace}>Удалить вкладку и ссылки</button>}>
        Вкладка «{active.name}» и её ссылки будут удалены. Приложения, файлы и папки останутся на диске.
        {error && <span className="dialog-error" role="alert">{error}</span>}
      </ConfirmDialog>}
      {pendingNavigation && <ConfirmDialog title="Несохранённые изменения" cancel={() => { if (!busy) setPendingNavigation(null); }}
        actions={<>
          <button disabled={busy} onClick={() => { setPendingNavigation(null); pendingNavigation(); }}>Отбросить</button>
          <button className="primary" disabled={busy || !!appearanceForm()?.querySelector(':invalid')} onClick={() => {
            saveAppearance(() => { setPendingNavigation(null); pendingNavigation(); });
          }}>Сохранить и перейти</button>
        </>}>
        Сохранить изменения оформления перед переходом?
        {error && <span className="dialog-error" role="alert">{error}</span>}
      </ConfirmDialog>}
    </main>
  );
}
