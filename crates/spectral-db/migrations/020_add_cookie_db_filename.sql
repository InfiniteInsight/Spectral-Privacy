-- Add cookie_db_filename column to browser_cookies table
-- This stores the filename of the cookie database (e.g., "Cookies", "cookies.sqlite")
-- to provide context about where the cookie was found

ALTER TABLE browser_cookies ADD COLUMN cookie_db_filename TEXT NOT NULL DEFAULT 'Cookies';
