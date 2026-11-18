import { useLayoutEffect, useRef, type ReactNode } from 'react';

export default function ConfirmDialog({ title, children, cancel, actions }: {
  title: string; children: ReactNode; cancel: () => void; actions: ReactNode;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  useLayoutEffect(() => {
    dialog.current?.showModal();
  }, []);

  return <dialog ref={dialog} className="confirmation" aria-labelledby="confirmation-title"
    onCancel={(event) => { event.preventDefault(); cancel(); }}>
    <h2 id="confirmation-title">{title}</h2>
    <p>{children}</p>
    <div className="toolbar">
      <button type="button" autoFocus onClick={cancel}>Отмена</button>
      {actions}
    </div>
  </dialog>;
}
