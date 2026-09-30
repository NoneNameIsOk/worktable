export interface Settings { labName: string; clusterHealthCheckUrl: string; autostart: boolean; startInTray: boolean; vpnAutoconnect: boolean; lastVpnProfile: string | null; updateEndpoint: string }
export interface Service { id: string; name: string; url: string; icon: string; requiresVpn: boolean; favorite: boolean; pinnedToSidebar: boolean; sortOrder: number; enabled: boolean }
export interface Node { id: string; displayName: string; sshHost: string; enabled: boolean; sortOrder: number }
export interface Todo { id: string; title: string; completed: boolean; dueDate: string | null }
export interface CalendarEvent { id: string; title: string; startsAt: string; endsAt: string; notes: string; remindAt: string | null; notifiedAt: string | null }
export interface SidebarItem { id: string; sortOrder: number }
export interface VpnProfile { id: string; name: string }
export type VpnState = 'Disconnected' | 'Connecting' | 'Connected' | 'Reconnecting' | 'Disconnecting' | 'Failed';
export interface VpnStatus { state: VpnState; profileId: string | null; message: string }
export interface Capability { available: boolean; source: string; detail: string; platform: string }
export interface Snapshot { settings: Settings; services: Service[]; nodes: Node[]; todos: Todo[]; calendarEvents: CalendarEvent[]; sidebarItems: SidebarItem[]; vpnProfiles: VpnProfile[] }
export type Mutation = { type: 'saveTodo'; value: Todo } | { type: 'saveEvent'; value: CalendarEvent } | { type: 'saveService'; value: Service } | { type: 'saveNode'; value: Node } | { type: 'saveSettings'; value: Settings } | { type: 'renameProfile'; value: VpnProfile } | { type: 'reorderSidebar'; value: string[] } | { type: 'deleteTodo' | 'deleteEvent' | 'deleteService' | 'deleteNode'; value: string };
export interface UpdateInfo { currentVersion: string; version: string; releaseNotes: string; downloadUrl: string | null; available: boolean }
