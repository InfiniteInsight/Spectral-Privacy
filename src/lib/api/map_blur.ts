import { invoke } from '@tauri-apps/api/core';

export interface MapBlurRequest {
	id: string;
	profileId: string;
	service: 'GoogleMaps' | 'AppleMaps' | 'BingMaps';
	status: 'URLGenerated' | 'Submitted' | 'Completed' | 'Failed';
	requestUrl: string;
	streetAddress: string;
	latitude: number | null;
	longitude: number | null;
	generatedAt: string;
	submittedAt?: string;
	completedAt?: string;
}

export const mapBlurAPI = {
	async generateRequests(vaultId: string, profileId: string): Promise<MapBlurRequest[]> {
		return await invoke<MapBlurRequest[]>('generate_map_blur_requests', {
			vaultId,
			profileId
		});
	},

	async getRequests(vaultId: string, profileId: string): Promise<MapBlurRequest[]> {
		return await invoke<MapBlurRequest[]>('get_map_blur_requests', {
			vaultId,
			profileId
		});
	},

	async markSubmitted(vaultId: string, requestId: string, service: string): Promise<void> {
		return await invoke('mark_map_blur_submitted', {
			vaultId,
			requestId,
			service
		});
	}
};
