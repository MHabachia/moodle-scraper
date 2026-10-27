import {act, renderHook, waitFor} from '@testing-library/react';
import {afterEach, describe, expect, it, vi} from 'vitest';
import {useUpdater} from './useUpdater';

const check = vi.fn();
const relaunch = vi.fn();
vi.mock('@tauri-apps/plugin-updater', () => ({check: () => check()}));
vi.mock('@tauri-apps/plugin-process', () => ({relaunch: () => relaunch()}));

afterEach(() => {
  vi.useRealTimers();
  check.mockReset();
  relaunch.mockReset();
});

describe('useUpdater', () => {
  it('meldet eine neue Version und installiert sie erst auf Wunsch', async () => {
    const downloadAndInstall = vi.fn().mockResolvedValue(undefined);
    check.mockResolvedValue({version: '2.0.0', downloadAndInstall});
    const {result} = renderHook(() => useUpdater(false));
    await act(() => result.current.checkNow());
    expect(result.current.state).toEqual({status: 'available', version: '2.0.0'});
    expect(downloadAndInstall).not.toHaveBeenCalled();
    await act(() => result.current.install());
    expect(downloadAndInstall).toHaveBeenCalledTimes(1);
    expect(relaunch).toHaveBeenCalledTimes(1);
  });

  it('zeigt „keine neue Version“, wenn nichts bereitsteht', async () => {
    check.mockResolvedValue(null);
    const {result} = renderHook(() => useUpdater(false));
    await act(() => result.current.checkNow());
    expect(result.current.state.status).toBe('none');
  });

  it('meldet Fehler nur bei manueller Suche', async () => {
    check.mockRejectedValue(new Error('offline'));
    const {result} = renderHook(() => useUpdater(false));
    await act(() => result.current.checkNow());
    expect(result.current.state).toEqual({status: 'error', message: 'offline'});
  });

  it('sucht beim Start still, aber nur wenn es eingeschaltet ist', async () => {
    vi.useFakeTimers();
    check.mockRejectedValue(new Error('offline'));
    renderHook(() => useUpdater(false));
    await act(() => vi.advanceTimersByTimeAsync(10_000));
    expect(check).not.toHaveBeenCalled();

    const {result} = renderHook(() => useUpdater(true));
    await act(() => vi.advanceTimersByTimeAsync(10_000));
    expect(check).toHaveBeenCalledTimes(1);
    expect(result.current.state.status).toBe('idle');
  });

  it('bricht die Installation mit Fehlermeldung ab', async () => {
    check.mockResolvedValue({version: '2.0.0', downloadAndInstall: vi.fn().mockRejectedValue(new Error('Signatur'))});
    const {result} = renderHook(() => useUpdater(false));
    await act(() => result.current.checkNow());
    await act(() => result.current.install());
    await waitFor(() => expect(result.current.state).toEqual({status: 'error', message: 'Signatur'}));
    expect(relaunch).not.toHaveBeenCalled();
  });
});
