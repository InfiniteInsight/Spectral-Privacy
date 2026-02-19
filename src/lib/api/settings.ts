import { invoke } from '@tauri-apps/api/core';

export async function testSmtpConnection(
	host: string,
	port: number,
	username: string,
	password: string
): Promise<void> {
	return await invoke('test_smtp_connection', { host, port, username, password });
}

export async function testImapConnection(
	host: string,
	port: number,
	username: string,
	password: string
): Promise<void> {
	return await invoke('test_imap_connection', { host, port, username, password });
}

export async function setPermissionPreset(vaultId: string, preset: string): Promise<void> {
	return await invoke('set_permission_preset', { vaultId, preset });
}

export async function getPermissionPreset(vaultId: string): Promise<string> {
	return await invoke('get_permission_preset', { vaultId });
}
