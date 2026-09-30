// @vitest-environment jsdom
import { afterEach, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import '@testing-library/jest-dom/vitest';
import { VpnCard } from './vpn';
afterEach(cleanup);
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
