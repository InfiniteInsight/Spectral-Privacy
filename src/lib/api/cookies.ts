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
	id?: string;
	cookieName: string;
	cookieDomain: string;
	browserType: string;
	profileName: string;
	cookieDbFilename: string;
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
	 * Remove all scanned cookies (both matched and unmatched).
	 */
	async removeAllCookies(vaultId: string): Promise<CookieRemovalResponse[]> {
		return await invoke<CookieRemovalResponse[]>('remove_all_cookies', { vaultId });
	},

	/**
	 * Remove all tracking cookies (only matched cookies).
	 */
	async removeAllTrackingCookies(vaultId: string): Promise<CookieRemovalResponse[]> {
		return await invoke<CookieRemovalResponse[]>('remove_all_tracking_cookies', { vaultId });
	},

	/**
	 * Remove a single cookie by its database ID.
	 */
	async removeSingleCookie(vaultId: string, cookieId: string): Promise<CookieRemovalResponse> {
		return await invoke<CookieRemovalResponse>('remove_single_cookie', { vaultId, cookieId });
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
	},

	/**
	 * Open the cookie's browser database location in file explorer.
	 */
	async openCookieLocation(browserType: string, profileName: string): Promise<void> {
		return await invoke('open_cookie_location', { browserType, profileName });
	}
};
