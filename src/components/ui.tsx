import { useEffect, useRef, type ReactNode } from 'react';
import { X, Plus } from 'lucide-react';
export function Modal({ title, children, onClose }: { title: string; children: ReactNode; onClose: () => void }) {
  const ref = useRef<HTMLDialogElement>(null);
  useEffect(() => { ref.current?.showModal(); }, []);
  return <dialog ref={ref} onCancel={onClose} className="modal"><div className="modal-title"><h2>{title}</h2><button className="icon-btn" aria-label="关闭" onClick={onClose}><X size={20} /></button></div>{children}</dialog>;
}
export function Empty({ title, text, action, onAction }: { title: string; text: string; action?: string; onAction?: () => void }) { return <div className="empty"><div className="empty-symbol">＋</div><strong>{title}</strong><p>{text}</p>{action && <button className="button secondary" onClick={onAction}><Plus size={15} />{action}</button>}</div>; }
export function Field({ label, children }: { label: string; children: ReactNode }) { return <label className="field"><span>{label}</span>{children}</label>; }
export function Toggle({ label, checked, onChange, hint }: { label: string; checked: boolean; onChange: (v: boolean) => void; hint?: string }) { return <label className="toggle-row"><span>{label}{hint && <small>{hint}</small>}</span><input type="checkbox" role="switch" checked={checked} onChange={e => onChange(e.target.checked)} /></label>; }
