// @vitest-environment jsdom
import { afterEach, expect, it, vi } from 'vitest';
import { cleanup, render, screen } from '@testing-library/react';
import '@testing-library/jest-dom/vitest';
import { WebService } from './web-service';
import type { Service } from '../types';
afterEach(() => { cleanup(); vi.unstubAllGlobals(); });
const service: Service = { id: 'test', name: 'ClearML', url: 'https://example.org', icon: '', enabled: true, requiresVpn: true, favorite: true, pinnedToSidebar: true, sortOrder: 0 };
it('offers an isolated browser tab only after backend access validation', async () => {
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: true, json: async () => service.url }));
  render(<WebService service={service} />);
  const link = await screen.findByRole('link', { name: '打开 ClearML' });
  expect(link).toHaveAttribute('href', service.url);
  expect(link).toHaveAttribute('target', '_blank'); expect(link).toHaveAttribute('rel', 'noopener noreferrer');
});
it('does not offer a service URL when the VPN/network check fails', async () => {
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: false, json: async () => ({ error: '实验室网络尚不可达。' }) }));
  render(<WebService service={service} />);
  expect(await screen.findByRole('alert')).toHaveTextContent('实验室网络尚不可达');
  expect(screen.queryByRole('link')).not.toBeInTheDocument();
});
