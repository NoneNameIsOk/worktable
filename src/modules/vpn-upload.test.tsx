// @vitest-environment jsdom
import { afterEach, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import '@testing-library/jest-dom/vitest';
import { VpnUpload } from './vpn-upload';
import { api } from '../lib/api';
afterEach(() => { cleanup(); vi.restoreAllMocks(); });
it('uploads multiple chosen files independently and retains each result', async () => {
  const upload = vi.spyOn(api, 'uploadVpn').mockResolvedValueOnce(true).mockRejectedValueOnce(new Error('配置包含不允许的指令。'));
  const refresh = vi.fn().mockResolvedValue(undefined);
  render(<VpnUpload refresh={refresh} />);
  const files = [new File(['client'], 'one.ovpn'), new File(['up script'], 'two.ovpn')];
  fireEvent.change(screen.getByLabelText('上传 VPN 配置文件'), { target: { files } });
  await screen.findByText(/配置包含不允许的指令/);
  expect(upload).toHaveBeenNthCalledWith(1, files[0]);
  expect(upload).toHaveBeenNthCalledWith(2, files[1]);
  expect(screen.getByText(/上传成功/)).toBeInTheDocument();
  await waitFor(() => expect(refresh).toHaveBeenCalledOnce());
});
it('accepts a dropped file without requiring an installed VPN backend', async () => {
  const upload = vi.spyOn(api, 'uploadVpn').mockResolvedValue(true);
  render(<VpnUpload refresh={async () => {}} />);
  const file = new File(['client'], 'dropped.ovpn');
  fireEvent.drop(screen.getByLabelText('上传 VPN 配置文件'), { dataTransfer: { files: [file] } });
  await screen.findByText(/上传成功/);
  expect(upload).toHaveBeenCalledWith(file);
});
