import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';

interface Action {
  label: string;
  run: () => void;
  danger?: boolean;
}

export default function ActionMenu({ label, actions, disabled, className }: {
  label: string; actions: Action[]; disabled?: boolean; className?: string;
}) {
  const trigger = useRef<HTMLButtonElement>(null);
  const menu = useRef<HTMLDivElement>(null);
  const [open, setOpen] = useState(false);

  function close(restore = true) {
    setOpen(false);
    if (restore) trigger.current?.focus();
  }

  useLayoutEffect(() => {
    if (!open || !trigger.current || !menu.current) return;
    const anchor = trigger.current.getBoundingClientRect();
    const popup = menu.current.getBoundingClientRect();
    const left = Math.max(8, Math.min(anchor.right - popup.width, window.innerWidth - popup.width - 8));
    const top = anchor.bottom + popup.height + 12 <= window.innerHeight
      ? anchor.bottom + 4 : Math.max(8, anchor.top - popup.height - 4);
    Object.assign(menu.current.style, { left: `${left}px`, top: `${top}px` });
    menu.current.querySelector('button')?.focus({ preventScroll: true });
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const outside = (event: PointerEvent) => {
      if (event.target instanceof Node && !menu.current?.contains(event.target)
        && !trigger.current?.contains(event.target)) close(false);
    };
    const moved = () => close();
    document.addEventListener('pointerdown', outside);
    window.addEventListener('resize', moved);
    document.addEventListener('scroll', moved, true);
    return () => {
      document.removeEventListener('pointerdown', outside);
      window.removeEventListener('resize', moved);
      document.removeEventListener('scroll', moved, true);
    };
  }, [open]);

  return <>
    <button type="button" className={className ?? 'card-extra'} ref={trigger} title={label}
      aria-label={label} aria-haspopup="menu" aria-expanded={open} disabled={disabled}
      onClick={() => setOpen(!open)}
      onKeyDown={(event) => {
        if (event.key === 'ArrowDown') { event.preventDefault(); setOpen(true); }
      }}>⋯</button>
    {open && createPortal(
      <div ref={menu} className="action-menu" role="menu" aria-label={label}
        onKeyDown={(event) => {
          if (event.key === 'Escape') { event.preventDefault(); close(); return; }
          if (event.key === 'Tab') { close(); return; }
          const buttons = Array.from(event.currentTarget.querySelectorAll('button'));
          const index = buttons.indexOf(document.activeElement as HTMLButtonElement);
          const next = { ArrowDown: (index + 1) % buttons.length,
            ArrowUp: (index + buttons.length - 1) % buttons.length, Home: 0, End: buttons.length - 1 }[event.key];
          if (next !== undefined) { event.preventDefault(); buttons[next]?.focus(); }
        }}>
        {actions.map((action) => (
          <button key={action.label} role="menuitem" tabIndex={-1} type="button"
            className={action.danger ? 'danger' : undefined} onClick={() => { close(); action.run(); }}>
            {action.label}
          </button>
        ))}
      </div>, document.body)}
  </>;
}
