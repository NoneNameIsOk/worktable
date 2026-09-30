import { DndContext, closestCenter, KeyboardSensor, PointerSensor, useSensor, useSensors, type DragEndEvent } from '@dnd-kit/core';
import { SortableContext, useSortable, verticalListSortingStrategy, sortableKeyboardCoordinates } from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import { Home, LayoutGrid, Server, FlaskConical, CalendarDays, Puzzle, Settings, GripVertical, ArrowUpRight, type LucideIcon } from 'lucide-react';
import type { Service, SidebarItem, VpnStatus } from '../types';
import { moduleNames, reorder, vpnLabels } from '../lib/logic';
const icons: Record<string, LucideIcon> = { home: Home, services: LayoutGrid, nodes: Server, experiments: FlaskConical, calendar: CalendarDays, plugins: Puzzle, settings: Settings };
function Item({ item, active, services, onNavigate }: { item: SidebarItem; active: string; services: Service[]; onNavigate: (id: string) => void }) {
  const { attributes, listeners, setNodeRef, transform, transition } = useSortable({ id: item.id });
  const service = services.find(s => `service:${s.id}` === item.id);
  const Icon = icons[item.id] ?? ArrowUpRight;
  return <div ref={setNodeRef} style={{ transform: CSS.Transform.toString(transform), transition }} className={`nav-row ${active === item.id ? 'active' : ''}`}>
    <button className="nav-link" onClick={() => onNavigate(item.id)} disabled={service && !service.enabled}><Icon size={18} /><span>{service?.name ?? moduleNames[item.id] ?? item.id}</span>{(item.id === 'experiments' || item.id === 'plugins') && <small>即将</small>}</button>
    <button className="drag-handle icon-btn" aria-label={`调整${service?.name ?? moduleNames[item.id]}顺序`} {...attributes} {...listeners}><GripVertical size={14} /></button>
  </div>;
}
export function Sidebar({ items, services, active, vpn, labName, navigate, onReorder }: { items: SidebarItem[]; services: Service[]; active: string; vpn: VpnStatus; labName: string; navigate: (id: string) => void; onReorder: (ids: string[]) => void }) {
  const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: 6 } }), useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }));
  function end(e: DragEndEvent) { if (e.over) onReorder(reorder(items, String(e.active.id), String(e.over.id))); }
  return <aside className="sidebar"><div className="brand"><div className="brand-mark">H<span>·</span></div><div><strong>Worktable</strong><small>HYKSJ LAB</small></div></div><div className="workspace-label">工作空间 <span>LOCAL</span></div>
    <nav aria-label="主导航"><DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={end}><SortableContext items={items.map(i => i.id)} strategy={verticalListSortingStrategy}>{items.map(item => <Item key={item.id} item={item} services={services} active={active} onNavigate={navigate} />)}</SortableContext></DndContext></nav>
    <div className="sidebar-bottom"><div><span className={`dot ${vpn.state === 'Connected' ? 'green' : ''}`} />VPN {vpnLabels[vpn.state]}</div><small>{labName}</small><div className="local-note">本地工作台 <span>v0.1.0</span></div></div>
  </aside>;
}
