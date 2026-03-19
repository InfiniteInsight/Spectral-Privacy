import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';

export const load: PageLoad = async () => {
	const { vaultStore } = await import('$lib/stores');

	if (!vaultStore.currentVaultId || !vaultStore.isCurrentVaultUnlocked) {
		throw redirect(302, '/');
	}

	const { profileStore } = await import('$lib/stores');
	await profileStore.loadProfiles(vaultStore.currentVaultId);

	// If no profile exists, redirect to setup
	if (profileStore.profiles.length === 0) {
		throw redirect(302, '/profile/setup');
	}

	const { profileAPI } = await import('$lib/api/profile');
	const profile = await profileAPI.get(vaultStore.currentVaultId, profileStore.profiles[0].id);

	return { profile };
};
