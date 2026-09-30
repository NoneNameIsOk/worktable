// @vitest-environment jsdom
import { afterEach, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import '@testing-library/jest-dom/vitest';
import { api } from '../lib/api';
import { VpnCard } from './vpn';
afterEach(() => { cleanup(); vi.restoreAllMocks(); });
const profiles = [{ id: 'one', name: 'lab-one' }, { id: 'two', name: 'lab-two' }];
it('shows all local profiles and lets the user select one', async () => {
  const select = vi.fn();
  render(<VpnCard profiles={profiles} selected="one" select={select} status={{ state: 'Disconnected', profileId: null, message: '' }} capability={null} run={async () => {}} refresh={async () => {}} mutate={async () => true} />);
  expect(screen.getByRole('radio', { name: /lab-one.ovpn/ })).toBeChecked();
  fireEvent.click(screen.getByRole('radio', { name: /lab-two.ovpn/ }));
  expect(select).toHaveBeenCalledWith('two');
});
it('prevents switching the profile while a connection is active', () => {
  render(<VpnCard profiles={profiles} selected="one" select={vi.fn()} status={{ state: 'Connected', profileId: 'one', message: '' }} capability={null} run={async () => {}} refresh={async () => {}} mutate={async () => true} />);
  expect(screen.getByRole('radio', { name: /lab-two.ovpn/ })).toBeDisabled();
  expect(screen.getByRole('button', { name: '删除配置 lab-one' })).toBeDisabled();
});

it('asks for credentials for an auth profile and passes them only on submit', async () => {
  vi.spyOn(api, 'authRequired').mockResolvedValue(true);
  const connect = vi.spyOn(api, 'connect').mockResolvedValue(undefined);
  render(<VpnCard profiles={profiles} selected="one" select={vi.fn()} status={{ state: 'Disconnected', profileId: null, message: '' }} capability={{ available: true, source: 'test', detail: '', platform: 'test' }} run={async fn => { await fn(); }} refresh={async () => {}} mutate={async () => true} />);
  HTMLDialogElement.prototype.showModal = vi.fn();
  fireEvent.click(screen.getByRole('button', { name: '连接 VPN' }));
  const username = await screen.findByLabelText('VPN 用户名');
  expect(connect).not.toHaveBeenCalled();
  fireEvent.change(username, { target: { value: 'test-user' } });
  const password = screen.getByLabelText('VPN 密码');
  fireEvent.change(password, { target: { value: 'test-only-password' } });
  fireEvent.submit(password.closest('form')!);
  expect(connect).toHaveBeenCalledWith('one', { username: 'test-user', password: 'test-only-password' });
  expect(screen.queryByLabelText('VPN 密码')).not.toBeInTheDocument();
});
