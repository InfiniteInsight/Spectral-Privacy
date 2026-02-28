/**
 * Utility functions for displaying broker information
 */

/**
 * Convert removal method from PascalCase to readable format
 */
export function getRemovalMethodDisplay(method: string): string {
	// Convert PascalCase to readable format
	return method.replace(/([A-Z])/g, ' $1').trim();
}

/**
 * Get display text and color for scan status
 */
export function getScanStatusDisplay(status: string | null): { text: string; color: string } {
	if (!status) {
		return { text: 'Not Scanned', color: 'text-gray-700 bg-gray-100' };
	}

	switch (status) {
		case 'Found':
			return { text: 'Found', color: 'text-red-700 bg-red-100' };
		case 'NotFound':
			return { text: 'Not Found', color: 'text-green-700 bg-green-100' };
		default:
			return { text: status, color: 'text-gray-700 bg-gray-100' };
	}
}

/**
 * Format date string to readable format
 */
export function formatDate(dateString: string): string {
	try {
		const date = new Date(dateString);
		return date.toLocaleDateString('en-US', {
			year: 'numeric',
			month: 'long',
			day: 'numeric'
		});
	} catch {
		return dateString;
	}
}
