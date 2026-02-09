/**
 * Class Name Utility
 *
 * Combines clsx and tailwind-merge for conditional class names.
 */

import { clsx, type ClassValue } from 'clsx';
import { twMerge } from 'tailwind-merge';

/**
 * Merge class names with Tailwind CSS conflict resolution
 *
 * @example
 * cn('px-2 py-1', condition && 'bg-red-500', 'px-4')
 * // Returns: 'py-1 bg-red-500 px-4' (px-4 overrides px-2)
 */
export function cn(...inputs: ClassValue[]): string {
	return twMerge(clsx(inputs));
}
