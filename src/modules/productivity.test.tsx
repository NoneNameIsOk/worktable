// @vitest-environment jsdom
import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import '@testing-library/jest-dom/vitest';
import { Calendar, TodoCard } from './productivity';
import type { Todo } from '../types';
beforeAll(() => { HTMLDialogElement.prototype.showModal = function () { this.open = true; }; });
afterEach(cleanup);
const todo: Todo = { id: '7e992e68-2e12-41ad-8c28-0c96e6377cb0', title: '整理实验记录', completed: false, dueDate: null };
describe('Todo UI', () => {
  it('adds a trimmed todo and clears only after a successful save', async () => { const mutate = vi.fn().mockResolvedValueOnce(false).mockResolvedValue(true); render(<TodoCard todos={[]} mutate={mutate} />); const input = screen.getByLabelText('新增 Todo'); fireEvent.change(input, { target: { value: '  阅读论文  ' } }); fireEvent.click(screen.getByText('添加')); await waitFor(() => expect(mutate).toHaveBeenCalledOnce()); expect(input).toHaveValue('  阅读论文  '); fireEvent.click(screen.getByText('添加')); await waitFor(() => expect(input).toHaveValue('')); expect(mutate.mock.calls[0][0].value.title).toBe('阅读论文'); });
  it('completes, edits, and deletes existing records', async () => { const mutate = vi.fn().mockResolvedValue(true); render(<TodoCard todos={[todo]} mutate={mutate} />); fireEvent.click(screen.getByLabelText(`完成 ${todo.title}`)); expect(mutate).toHaveBeenCalledWith({ type: 'saveTodo', value: { ...todo, completed: true } }); fireEvent.click(screen.getByLabelText(`编辑 ${todo.title}`)); fireEvent.change(screen.getByLabelText('事项'), { target: { value: '补充结论' } }); fireEvent.click(screen.getByText('保存')); await waitFor(() => expect(mutate).toHaveBeenCalledWith({ type: 'saveTodo', value: { ...todo, title: '补充结论' } })); fireEvent.click(screen.getByLabelText(`删除 ${todo.title}`)); expect(mutate).toHaveBeenCalledWith({ type: 'deleteTodo', value: todo.id }); });
  it('keeps the editor open on a rejected save', async () => { const mutate = vi.fn().mockResolvedValue(false); render(<TodoCard todos={[todo]} mutate={mutate} />); fireEvent.click(screen.getByLabelText(`编辑 ${todo.title}`)); fireEvent.click(screen.getByText('保存')); await waitFor(() => expect(mutate).toHaveBeenCalled()); expect(screen.getByRole('dialog')).toBeVisible(); });
});
describe('Calendar UI', () => { it('creates a schedule with optional reminder', async () => { const mutate = vi.fn().mockResolvedValue(true); render(<Calendar events={[]} mutate={mutate} />); fireEvent.click(screen.getByText('新建日程')); fireEvent.change(screen.getByLabelText('标题'), { target: { value: '每周组会' } }); fireEvent.click(screen.getByText('保存日程')); await waitFor(() => expect(mutate).toHaveBeenCalled()); expect(mutate.mock.calls[0][0].value).toMatchObject({ title: '每周组会', remindAt: null, notifiedAt: null }); }); });
