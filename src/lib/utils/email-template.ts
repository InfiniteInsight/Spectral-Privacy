import { profileStore } from '$lib/stores/profile.svelte';
import type { EmailTemplate } from '$lib/api/brokers';

/**
 * Substitute template variables with actual profile data.
 * Variables: {user_email}, {full_name}, {first_name}, {last_name},
 * {address}, {city}, {state}, {zip_code}, {ip_address}
 */
export function substituteEmailVariables(template: EmailTemplate): {
	email: string;
	subject: string;
	body: string;
} {
	const profile = profileStore.currentProfile;

	if (!profile) {
		// Return template with placeholders if no profile
		return {
			email: template.email,
			subject: template.subject,
			body: template.body
		};
	}

	// Construct full name from parts
	const fullName =
		[profile.first_name, profile.middle_name, profile.last_name].filter(Boolean).join(' ') ||
		'[Your Name]';

	const replacements: Record<string, string> = {
		'{user_email}': profile.email || '[Your Email]',
		'{full_name}': fullName,
		'{first_name}': profile.first_name || '[Your First Name]',
		'{last_name}': profile.last_name || '[Your Last Name]',
		'{address}': profile.address_line1 || '[Your Address]',
		'{city}': profile.city || '[Your City]',
		'{state}': profile.state || '[Your State]',
		'{zip_code}': profile.zip_code || '[Your ZIP]',
		'{ip_address}': '[Your IP Address]' // Runtime value
	};

	let substitutedSubject = template.subject;
	let substitutedBody = template.body;

	// Replace all variables
	for (const [variable, value] of Object.entries(replacements)) {
		substitutedSubject = substitutedSubject.replaceAll(variable, value);
		substitutedBody = substitutedBody.replaceAll(variable, value);
	}

	return {
		email: template.email,
		subject: substitutedSubject,
		body: substitutedBody
	};
}
