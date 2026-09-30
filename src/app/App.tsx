import { useCallback, useEffect, useRef, useState } from 'react';
import { FlaskConical, Puzzle, ArrowRight, LayoutGrid, RefreshCw, X, AlertCircle, PanelLeft, LockKeyhole } from 'lucide-react';
import { Sidebar } from '../components/Sidebar';
import { Empty, Modal } from '../components/ui';
import { Calendar, TodoCard } from '../modules/productivity';
import { Services, ServiceCards } from '../modules/services';
import { Nodes } from '../modules/nodes';
import { Settings } from '../modules/settings';
import { WebService } from '../modules/web-service';
import { WebReminders } from '../modules/reminders';
import { VpnCard } from '../modules/vpn';
import { api, desktop } from '../lib/api';
import { errorMessage, moduleNames, serviceAccess, vpnLabels } from '../lib/logic';
import type { Capability, Mutation, Service, Snapshot, VpnStatus } from '../types';
export function App() {
  const [data, setData] = useState<Snapshot | null>(null); const [active, setActive] = useState('home'); const [error, setError] = useState(''); const [busy, setBusy] = useState(false);
  const [vpn, setVpn] = useState<VpnStatus>({ state: 'Disconnected', profileId: null, message: 'VPN 未连接，本地功能可正常使用。' });
  const [capability, setCapability] = useState<Capability | null>(null); const [selected, setSelected] = useState(''); const [pending, setPending] = useState<Service | null>(null);
  const [vpnPrompt, setVpnPrompt] = useState(false); const surface = useRef<HTMLDivElement>(null); const currentService = useRef<string | null>(null);
  const refresh = useCallback(async () => { const snapshot = await api.snapshot(); setData(snapshot); setSelected(old => snapshot.vpnProfiles.some(p => p.id === old) ? old : snapshot.settings.lastVpnProfile ?? snapshot.vpnProfiles[0]?.id ?? ''); }, []);
  const run = useCallback(async (fn: () => Promise<unknown>) => { setBusy(true); setError(''); try { await fn(); } catch (e) { setError(errorMessage(e)); } finally { setBusy(false); } }, []);
  // A failed save leaves the dialog open and reports the error.
  const mutate = useCallback(async (m: Mutation) => { setError(''); try { setData(await api.mutate(m)); return true; } catch (e) { setError(errorMessage(e)); return false; } }, []);
  useEffect(() => { const load = async () => { try { await refresh(); setCapability(await api.vpnCapability()); } catch (e) { setError(errorMessage(e)); } }; void load(); }, [refresh]);
  useEffect(() => { let disposed = false; const poll = async () => { try { const s = await api.vpnStatus(); if (!disposed) { setVpn(s); if (s.state === 'Connected' && pending) { setVpnPrompt(false); setPending(null); setActive(`service:${pending.id}`); } } } catch (e) { if (!disposed) setError(errorMessage(e)); } }; void poll(); const timer = window.setInterval(() => { void poll(); }, 1000); return () => { disposed = true; clearInterval(timer); }; }, [pending]);
  const navigate = useCallback((id: string) => {
    if (!id.startsWith('service:')) { currentService.current = null; void run(async () => { await api.hideServices(); setActive(id); }); }
    else { const s = data?.services.find(s => `service:${s.id}` === id); if (s) { if (!desktop) { if (!s.enabled) setError('此服务已禁用。'); else setActive(id); return; } const access = serviceAccess(s, vpn.state); if (access === 'vpn') { void run(async () => { await api.hideServices(); currentService.current = null; setActive('home'); setPending(s); setVpnPrompt(true); }); } else if (access === 'open') setActive(id); else setError('此服务已禁用。'); } }
  }, [data, vpn.state, run]);
  const open = (s: Service) => navigate(`service:${s.id}`);
  useEffect(() => {
    if (!desktop || !active.startsWith('service:')) return;
    const s = data?.services.find(s => `service:${s.id}` === active); if (!s || !surface.current) return;
    let disposed = false; const host = surface.current;
    const bounds = () => { const r = host.getBoundingClientRect(); return { x: r.x, y: r.y, width: r.width, height: r.height }; };
    if (s.requiresVpn && vpn.state !== 'Connected') { void api.hideServices().catch(e => setError(errorMessage(e))); currentService.current = null; return; }
    void api.openService(s.id, bounds()).then(() => { if (!disposed) currentService.current = s.id; }).catch(e => { if (!disposed) setError(errorMessage(e)); });
    const observer = new ResizeObserver(() => { if (currentService.current === s.id) void api.resizeService(bounds()).catch(e => setError(errorMessage(e))); }); observer.observe(host);
    return () => { disposed = true; observer.disconnect(); };
  }, [active, data?.services, vpn.state]);
  // Both runtimes load real persistent data through their transport adapter.
  if (!data) return <div className="startup"><div className="brand-mark">H<span>·</span></div><h1>HYKSJ Worktable</h1><p>{error || '正在打开本地工作空间…'}</p>{error && <button className="button primary" onClick={() => { void run(async () => { await refresh(); setCapability(await api.vpnCapability()); }); }}>重试</button>}{!desktop && <code>npm run dev</code>}</div>;
  const selectedService = data.services.find(s => `service:${s.id}` === active);
  const vpnProps = { profiles: data.vpnProfiles, status: vpn, capability, selected, select: setSelected, run, refresh, mutate };
  const today = new Intl.DateTimeFormat('zh-CN', { year: 'numeric', month: 'long', day: 'numeric', weekday: 'long' }).format(new Date());
  const favorite = data.services.filter(s => s.favorite && s.enabled);
  return <div className="app-shell"><Sidebar items={data.sidebarItems} services={data.services} active={active} vpn={vpn} labName={data.settings.labName} navigate={navigate} onReorder={ids => { void run(() => mutate({ type: 'reorderSidebar', value: ids })); }} /><div className="workspace"><header className="topbar"><div><PanelLeft size={16} /><span>{data.settings.labName}</span><span className="breadcrumb">/</span><strong>{selectedService?.name ?? moduleNames[active]}</strong></div><div><span className={`dot ${vpn.state === 'Connected' ? 'green' : ''}`} /><span>VPN {vpnLabels[vpn.state]}</span><span className="top-divider" /><span className="local-chip">{desktop ? "桌面版" : "网页版"}</span></div></header>
    {error && <div className="error-banner" role="alert"><AlertCircle size={17} /><span>{error}</span><button className="icon-btn" aria-label="关闭错误提示" onClick={() => setError('')}><X size={16} /></button></div>}
    <main className={selectedService && desktop ? 'service-workspace' : 'main-content'} aria-busy={busy}>
      {!selectedService && <div className="page-heading"><div><div className="eyebrow">{active === 'home' ? 'YOUR RESEARCH, IN ONE PLACE' : 'HYKSJ WORKTABLE'}</div><h1>{active === 'home' ? '让科研工作，更有条理。' : moduleNames[active]}</h1><p>{active === 'home' ? today : { services: '你的实验室服务入口', nodes: '从这里，开始远程工作', calendar: '为专注的工作留出时间', experiments: '连接每一次探索', plugins: '为未来的工作方式留出空间', settings: '让工作台适合你的习惯' }[active]}</p></div>{active === 'home' && <div className="date-badge"><span>{new Date().getMonth() + 1}月</span><strong>{new Date().getDate()}</strong></div>}</div>}
      {active === 'home' && <div className="dashboard"><VpnCard {...vpnProps} /><div className="dashboard-columns"><Calendar events={data.calendarEvents} mutate={mutate} compact /><TodoCard todos={data.todos} mutate={mutate} /></div><section className="card favorites-card"><div className="card-head"><h2><LayoutGrid size={18} />常用服务</h2><button className="text-btn" onClick={() => navigate('services')}>管理服务<ArrowRight size={14} /></button></div>{favorite.length ? <ServiceCards services={favorite} open={open} /> : <Empty title="把常用工具放在手边" text="添加服务并设为常用，即可从首页快速访问。" action="配置服务" onAction={() => navigate('services')} />}</section></div>}
      {active === 'services' && <Services services={data.services} mutate={mutate} open={open} />}
      {active === 'nodes' && <Nodes nodes={data.nodes} mutate={mutate} run={run} />}
      {active === 'calendar' && <Calendar events={data.calendarEvents} mutate={mutate} />}
      {active === 'settings' && <Settings key={data.settings.labName + data.settings.clusterHealthCheckUrl + data.settings.updateEndpoint} settings={data.settings} mutate={mutate} run={run} refresh={refresh} navigate={navigate} vpnCard={<VpnCard {...vpnProps} management />} />}
      {(active === 'experiments' || active === 'plugins') && <section className="card coming-soon">{active === 'experiments' ? <FlaskConical size={42} /> : <Puzzle size={42} />}<span className="tag">即将推出</span><h2>{active === 'experiments' ? '实验监控' : '插件与扩展'}</h2><p>{active === 'experiments' ? '未来将在这里查看实验运行状态、节点 / GPU 关联和日志进度，并与 ClearML 集成。' : '未来将在这里扩展工作台能力，提供插件 API 与权限管理。'}</p><small>此模块尚未实现，当前版本不提供相关运行能力。</small></section>}
      {selectedService && !desktop && <WebService key={selectedService.id} service={selectedService} />}
      {selectedService && desktop && <><div className="service-toolbar"><div><strong>{selectedService.name}</strong><span>{selectedService.url}</span></div><button className="button secondary" onClick={() => { void run(async () => { await api.resetService(selectedService.id); const r = surface.current?.getBoundingClientRect(); if (r) await api.openService(selectedService.id, { x: r.x, y: r.y, width: r.width, height: r.height }); }); }}><RefreshCw size={14} />重新打开</button></div><div className="service-hint">页面由服务自身提供。连接异常时可重新打开；新窗口登录流程暂不支持。</div><div ref={surface} className="service-surface">{selectedService.requiresVpn && vpn.state !== 'Connected' ? <div className="empty"><LockKeyhole size={30} /><h2>需要连接实验室 VPN</h2><button className="button primary" onClick={() => { setPending(selectedService); setVpnPrompt(true); }}>连接 VPN</button></div> : <div className="webview-loading">正在打开服务 WebView…</div>}</div></>}
    </main><footer className="workspace-footer"><span>HYKSJ Worktable</span><span>专注科研 · 数据保存在本机</span></footer></div>
    {!desktop && <WebReminders />}
    {vpnPrompt && <Modal title="连接实验室网络" onClose={() => { setVpnPrompt(false); setPending(null); }}><p>「{pending?.name}」需要 VPN，连接成功后将自动打开。</p><VpnCard {...vpnProps} /></Modal>}
  </div>;
}
