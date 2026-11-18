import { useEffect, useState, type CSSProperties, type ReactNode } from 'react';
import { cardAccent, iconAccent, type Appearance, type ItemKind } from './accents';

export interface Application {
  id: string;
  name: string;
  icon: string | null;
}

export function Card({
  name, kind, appId, icon, settings, label, action, extra, disabled, detail,
}: {
  name: string; kind: ItemKind; appId?: string; icon?: string | null; settings: Appearance;
  label?: string; action?: () => void; extra?: ReactNode; disabled?: boolean; detail?: string;
}) {
  const [sample, setSample] = useState<{ source: string; color: string }>();
  const [failedSource, setFailedSource] = useState<string>();
  const currentIcon = icon && icon !== failedSource ? icon : undefined;
  useEffect(() => {
    let active = true;
    if (currentIcon) {
      void iconAccent(currentIcon).then((color) => {
        if (active) setSample({ source: currentIcon, color });
      });
    }
    return () => { active = false; };
  }, [currentIcon]);

  const sampled = sample && sample.source === currentIcon ? sample.color : undefined;
  const style = { '--card-accent': cardAccent(settings, kind, appId, sampled) } as CSSProperties;
  const content = (
    <>
      <span className={`item-icon ${currentIcon ? 'system-icon' : kind}`} aria-hidden="true">
        {currentIcon ? <img src={currentIcon} alt="" onError={() => setFailedSource(currentIcon)} /> : <span />}
      </span>
      <span className="card-copy">
        <span className="card-name" title={name}>{name}</span>
        {detail && <span className="muted">{detail}</span>}
      </span>
    </>
  );
  return (
    <div className="card" style={style}>
      {action
        ? <button className="card-main" onClick={action} disabled={disabled} aria-label={`${label} ${name}`}>{content}</button>
        : <div className="card-main">{content}</div>}
      {extra}
    </div>
  );
}
