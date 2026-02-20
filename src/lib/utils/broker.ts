/**
 * Get Tailwind color classes for difficulty badges
 */
export function getDifficultyColor(difficulty: string): string {
	switch (difficulty) {
		case 'Easy':
			return 'text-green-700 bg-green-100';
		case 'Medium':
			return 'text-yellow-700 bg-yellow-100';
		case 'Hard':
			return 'text-red-700 bg-red-100';
		default:
			return 'text-gray-700 bg-gray-100';
	}
}

/**
 * Get human-readable category display name
 */
export function getCategoryDisplay(category: string): string {
	// Convert PascalCase to readable format
	return category.replace(/([A-Z])/g, ' $1').trim();
}
