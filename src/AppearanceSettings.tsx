import { useCallback, useEffect, useState } from 'react';
import { Card, type Application } from './cards';
import { validHex, type Appearance } from './accents';

function ColorField({ label, value, onChange, onValidity }: {
  label: string; value: string; onChange: (color: string) => void;
  onValidity: (label: string, valid: boolean) => void;
}) {
  const [partial, setPartial] = useState<string | null>(null);
  const hex = partial ?? value;
  const valid = validHex(hex);
  useEffect(() => {
    onValidity(label, valid);
    return () => { onValidity(label, true); };
  }, [label, onValidity, valid]);
  return (
    <div className="color-field">
      <span>{label}</span>
      <div className="toolbar">
        <input type="color" aria-label={`${label} — палитра`} value={value}
          onChange={(event) => { setPartial(null); onChange(event.target.value); }} />
        <input type="text" className="hex" aria-label={`${label} — HEX`} maxLength={7} value={hex}
          aria-invalid={!valid} required pattern="#[0-9a-fA-F]{6}"
          aria-describedby={!valid ? `hex-error-${label.replaceAll(" ", "-")}` : undefined}
          onChange={(event) => {
            const next = event.target.value;
            setPartial(validHex(next) ? null : next);
            if (validHex(next)) onChange(next);
          }} />
      </div>
      {!valid && <small id={`hex-error-${label.replaceAll(" ", "-")}`} className="danger">Введите цвет в формате #RRGGBB.</small>}
    </div>
  );
}

export default function AppearanceSettings({ draft, change, applications, save, cancel, busy, dirty }: {
  draft: Appearance; change: (next: Appearance) => void; applications: Application[];
  save: () => void; cancel: () => void; busy: boolean; dirty: boolean;
}) {
  const [selected, setSelected] = useState('');
  const [invalid, setInvalid] = useState<string[]>([]);
  const validity = useCallback((label: string, valid: boolean) => {
    setInvalid((fields) => valid
      ? fields.includes(label) ? fields.filter((field) => field !== label) : fields
      : fields.includes(label) ? fields : [...fields, label]);
  }, []);
  const app = applications.find(({ id }) => id === selected) ?? applications[0];
  const override = app && Object.hasOwn(draft.overrides, app.id) ? draft.overrides[app.id] : undefined;

  return (
    <div className="appearance">
      <form id="appearance-form" className="settings-form" onSubmit={(event) => { event.preventDefault(); save(); }}>
        <fieldset disabled={busy}>
          <legend>Цвет карточек</legend>
          <label>Режим
            <select value={draft.mode} onChange={(event) => change({ ...draft, mode: event.target.value === 'single' ? 'single' : 'icons' })}>
              <option value="icons">Из иконок</option>
              <option value="single">Один цвет</option>
            </select>
          </label>
          {draft.mode === 'single' && <ColorField onValidity={validity} label="Общий акцент" value={draft.accent} onChange={(accent) => change({ ...draft, accent })} />}
          {app && <>
            <label>Приложение
              <select value={app.id} onChange={(event) => setSelected(event.target.value)}>
                {applications.map((item) => <option key={item.id} value={item.id}>{item.name} · {item.id}</option>)}
              </select>
            </label>
            <ColorField onValidity={validity} key={app.id} label="Индивидуальный цвет" value={override ?? draft.accent}
              onChange={(color) => change({ ...draft, overrides: { ...draft.overrides, [app.id]: color } })} />
            <button type="button" disabled={!override} onClick={() => {
              const overrides = { ...draft.overrides };
              delete overrides[app.id];
              change({ ...draft, overrides });
            }}>Сбросить цвет приложения</button>
            <p className="muted">{override ? 'Используется индивидуальный цвет.' : 'Используется цвет по общему режиму.'}</p>
          </>}
        </fieldset>
        <fieldset disabled={busy}>
          <legend>Подсветка панели</legend>
          <label className="check"><input type="checkbox" checked={draft.glow} onChange={(event) => change({ ...draft, glow: event.target.checked })} /> Внутреннее свечение</label>
          <ColorField onValidity={validity} label="Цвет подсветки" value={draft.glowColor} onChange={(glowColor) => change({ ...draft, glowColor })} />
        </fieldset>
        <div className="toolbar">
          <button className="primary" disabled={busy || !dirty || invalid.length > 0}>Сохранить</button>
          <button type="button" disabled={busy || (!dirty && !invalid.length)} onClick={cancel}>Отменить изменения</button>
        </div>
      </form>
      <section className="preview" aria-label="Предпросмотр">
        <h2>Предпросмотр</h2>
        {app && <Card name={app.name} kind="application" appId={app.id} icon={app.icon} settings={draft} detail={override ? "Индивидуальный цвет" : draft.mode === "single" ? "Общий цвет" : undefined} />}
        <Card name="Notes.txt" kind="file" settings={draft} detail={draft.mode === "single" ? "Общий цвет" : undefined} />
        <Card name="Референсы" kind="folder" settings={draft} detail={draft.mode === "single" ? "Общий цвет" : undefined} />
      </section>
    </div>
  );
}
