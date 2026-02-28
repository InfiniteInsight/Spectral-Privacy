/**
 * Cookie scanning and removal API.
 */

import { invoke } from '@tauri-apps/api/core';

/**
 * Cookie scan response from backend.
 */
export interface CookieScanResponse {
	scanId: string;
	totalCookies: number;
	matchedCookies: number;
	cookiesByBrowser: Record<string, number>;
	cookiesByBroker: Record<string, number>;
	browsersScanned: string[];
	timestamp: string;
}

/**
 * Scanned cookie details.
 */
export interface ScannedCookie {
	cookieName: string;
	cookieDomain: string;
	browserType: string;
	profileName: string;
	matchedBrokerId: string | null;
	isSecure: boolean;
	isHttponly: boolean;
	creationTime: number | null;
	expiryTime: number | null;
}

/**
 * Cookie removal operation result.
 */
export interface CookieRemovalResponse {
	browserType: string;
	profileName: string;
	cookiesRemoved: number;
	cookiesFailed: number;
	backupPath: string | null;
	errors: string[];
}

/**
 * Cookie scanning and removal operations.
 */
export const cookiesAPI = {
	/**
	 * Scan all installed browsers for tracking cookies.
	 */
	async scanCookies(vaultId: string): Promise<CookieScanResponse> {
		return await invoke<CookieScanResponse>('scan_cookies', { vaultId });
	},

	/**
	 * Get all cookies matched to a specific broker.
	 */
	async getCookiesForBroker(vaultId: string, brokerId: string): Promise<ScannedCookie[]> {
		return await invoke<ScannedCookie[]>('get_cookies_for_broker', { vaultId, brokerId });
	},

	/**
	 * Remove all cookies for a specific broker.
	 */
	async removeCookiesForBroker(
		vaultId: string,
		brokerId: string
	): Promise<CookieRemovalResponse[]> {
		return await invoke<CookieRemovalResponse[]>('remove_cookies_for_broker', {
			vaultId,
			brokerId
		});
	},

	/**
	 * Get recent cookie scan history.
	 */
	async getRecentCookieScans(vaultId: string, limit: number): Promise<CookieScanResponse[]> {
		return await invoke<CookieScanResponse[]>('get_recent_cookie_scans', { vaultId, limit });
	},

	/**
	 * Get unmatched cookies from the most recent scan.
	 */
	async getUnmatchedCookies(vaultId: string): Promise<ScannedCookie[]> {
		return await invoke<ScannedCookie[]>('get_unmatched_cookies', { vaultId });
	}
};
