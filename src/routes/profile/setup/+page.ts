import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';

export const load: PageLoad = async () => {
	const { vaultStore } = await import('$lib/stores');

	// Check if vault is unlocked
	if (!vaultStore.currentVaultId || !vaultStore.isCurrentVaultUnlocked) {
		throw redirect(302, '/');
	}

	// Load profiles to check if one exists
	const { profileStore } = await import('$lib/stores');
	await profileStore.loadProfiles();

	// If profile exists, redirect to dashboard
	if (profileStore.profiles.length > 0) {
		throw redirect(302, '/');
	}

	return {};
};
