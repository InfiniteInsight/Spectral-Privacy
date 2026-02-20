#!/usr/bin/env node
/**
 * Cross-platform dev script launcher
 * Detects OS and runs the appropriate script (dev.sh or dev.ps1)
 */

const { spawn } = require('child_process');
const path = require('path');

const isWindows = process.platform === 'win32';
const scriptDir = __dirname;

let command, args, shell;

if (isWindows) {
	// Windows: Run PowerShell script
	command = 'powershell.exe';
	args = ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', path.join(scriptDir, 'dev.ps1')];
	shell = false;
} else {
	// Unix: Run bash script
	command = path.join(scriptDir, 'dev.sh');
	args = [];
	shell = false;
}

console.log(`Detected OS: ${process.platform}`);
console.log(`Running: ${command} ${args.join(' ')}\n`);

// Spawn the appropriate script
const child = spawn(command, args, {
	stdio: 'inherit',
	shell: shell,
	cwd: path.resolve(scriptDir, '..')
});

// Forward exit code
child.on('exit', (code) => {
	process.exit(code || 0);
});

// Handle errors
child.on('error', (err) => {
	console.error(`Failed to start dev script: ${err.message}`);
	process.exit(1);
});
