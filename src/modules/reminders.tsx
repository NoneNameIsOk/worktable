import { useEffect, useState } from 'react';
import { Bell, X } from 'lucide-react';
import { api, desktop } from '../lib/api';

export function WebReminders() {
  const [reminders, setReminders] = useState<[string, string][]>([]);
  const [offline, setOffline] = useState(false);
  useEffect(() => {
    if (desktop) return;
    let stopped = false;
    let inFlight = false;
    const check = async () => {
      // Claim only in a visible page, unless the browser can show OS notifications.
      if (stopped || inFlight || (document.hidden && !('Notification' in window && Notification.permission === 'granted'))) return;
      inFlight = true;
      try {
        const due = await api.claimReminders();
        if (!stopped) {
          setOffline(false);
          setReminders(old => [...old, ...due.filter(([id]) => !old.some(([oldId]) => oldId === id))]);
          if ('Notification' in window && Notification.permission === 'granted') {
            for (const [id, title] of due) {
              try { new Notification('Worktable 日程提醒', { body: title, tag: `worktable-${id}` }); } catch { /* The in-page reminder remains visible. */ }
            }
          }
        }
      } catch { if (!stopped) setOffline(true); }
      finally { inFlight = false; }
    };
    void check();
    const timer = window.setInterval(() => { void check(); }, 15000);
    const visible = () => { if (!document.hidden) void check(); };
    document.addEventListener('visibilitychange', visible);
    return () => { stopped = true; clearInterval(timer); document.removeEventListener('visibilitychange', visible); };
  }, []);
  return <div className="web-reminders" aria-live="polite">
    {offline && <div className="notice">后端暂不可达，日程提醒将在连接恢复后继续检查。</div>}
    {reminders.map(([id, title]) => <div className="reminder-toast" key={id}><Bell size={18} /><div><strong>日程提醒</strong><p>{title}</p></div><button className="icon-btn" aria-label={`关闭提醒 ${title}`} onClick={() => setReminders(old => old.filter(([item]) => item !== id))}><X size={16} /></button></div>)}
  </div>;
}
