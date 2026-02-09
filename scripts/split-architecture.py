#!/usr/bin/env python3
"""
Split the monolithic architecture.md into separate section files.
"""

import re
from pathlib import Path

PROJECT_ROOT = Path(__file__).parent.parent
ARCH_FILE = PROJECT_ROOT / "architecture.md"
ARCH_DIR = PROJECT_ROOT / "architecture"

# Section mapping: number -> (filename, title)
SECTIONS = {
    0: ("00-overview.md", "Overview"),
    1: ("01-vision.md", "Vision & Problem Statement"),
    2: ("02-high-level-architecture.md", "High-Level Architecture"),
    3: ("03-core-modules.md", "Core Modules — Detailed Design"),
    4: ("04-pii-discovery.md", "Local PII Discovery Engine"),
    5: ("05-network-telemetry.md", "Network Telemetry Engine"),
    6: ("06-removal-verification.md", "Removal Verification & Follow-Up Engine"),
    7: ("07-mail-communication.md", "Third-Party Communication Engine"),
    8: ("08-permissions.md", "Granular Permission System"),
    9: ("09-security.md", "Security Architecture"),
    10: ("10-reporting-dashboard.md", "Reporting & Progress Dashboard"),
    11: ("11-llm-integration.md", "LLM Integration Strategy"),
    12: ("12-frontend.md", "Frontend Architecture"),
    13: ("13-project-structure.md", "Project Structure"),
    14: ("14-database-schema.md", "Database Schema"),
    15: ("15-dependencies.md", "Dependencies"),
    16: ("16-broker-maintenance.md", "Broker Database Maintenance"),
    17: ("17-roadmap.md", "Development Roadmap"),
    18: ("18-license.md", "License Recommendation"),
    19: ("19-open-questions.md", "Open Questions & Discussion Points"),
    20: ("20-onboarding.md", "User Onboarding & PII Profile Setup"),
    21: ("21-geolocation-jurisdiction.md", "Geolocation & Jurisdiction System"),
    22: ("22-proactive-scanning.md", "Proactive Broker Scanning Model"),
    23: ("23-commercial-relationships.md", "Commercial Relationship Engine"),
    24: ("24-resolved-questions.md", "Resolved Open Questions"),
}


def split_architecture():
    """Split architecture.md into separate files."""
    ARCH_DIR.mkdir(exist_ok=True)

    content = ARCH_FILE.read_text()
    lines = content.split('\n')

    # Find section boundaries
    section_starts = []
    for i, line in enumerate(lines):
        if match := re.match(r'^## (\d+)\. ', line):
            section_num = int(match.group(1))
            section_starts.append((i, section_num))

    # Add end marker
    section_starts.append((len(lines), 999))

    # Extract header (before section 1)
    header_end = section_starts[0][0] if section_starts else len(lines)
    header_lines = lines[:header_end]

    # Write overview (header content)
    overview_content = '\n'.join(header_lines)
    (ARCH_DIR / SECTIONS[0][0]).write_text(overview_content)
    print(f"Created: {SECTIONS[0][0]} ({len(header_lines)} lines)")

    # Extract each section
    for idx, (start_line, section_num) in enumerate(section_starts[:-1]):
        end_line = section_starts[idx + 1][0]
        section_lines = lines[start_line:end_line]

        if section_num in SECTIONS:
            filename, title = SECTIONS[section_num]
            section_content = '\n'.join(section_lines)
            (ARCH_DIR / filename).write_text(section_content)
            print(f"Created: {filename} ({len(section_lines)} lines)")

    # Create index file
    create_index()

    print(f"\nDone! Created {len(SECTIONS)} files in {ARCH_DIR}")


def create_index():
    """Create an index file linking all sections."""
    index_lines = [
        "# Spectral Architecture Documentation",
        "",
        "This directory contains the Spectral architecture documentation, split into sections for easier navigation.",
        "",
        "## Sections",
        "",
    ]

    for num in sorted(SECTIONS.keys()):
        filename, title = SECTIONS[num]
        if num == 0:
            index_lines.append(f"- [{title}]({filename}) - Document header, version info, table of contents")
        else:
            index_lines.append(f"- [{num}. {title}]({filename})")

    index_lines.extend([
        "",
        "## Quick Reference",
        "",
        "| Topic | Sections |",
        "|-------|----------|",
        "| Vault & Encryption | [3](03-core-modules.md), [9](09-security.md) |",
        "| Broker Engine | [3](03-core-modules.md), [16](16-broker-maintenance.md), [22](22-proactive-scanning.md) |",
        "| Browser Automation | [3](03-core-modules.md) |",
        "| LLM & Prompt Safety | [3](03-core-modules.md), [11](11-llm-integration.md) |",
        "| Permissions | [8](08-permissions.md) |",
        "| User Onboarding | [20](20-onboarding.md) |",
        "| Jurisdiction & Legal | [21](21-geolocation-jurisdiction.md) |",
        "| Commercial Deletions | [23](23-commercial-relationships.md) |",
        "| Resolved Decisions | [24](24-resolved-questions.md) |",
        "",
        "## Version",
        "",
        "Architecture Document v0.3",
        "",
    ])

    (ARCH_DIR / "README.md").write_text('\n'.join(index_lines))
    print("Created: README.md (index)")


if __name__ == "__main__":
    split_architecture()
