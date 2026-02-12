#!/usr/bin/env python3
"""
Validate broker definition TOML files against the schema.

This script is called by pre-commit to ensure broker definitions are valid.
"""

import sys
import tomllib  # Python 3.11+
from pathlib import Path
from typing import Any

# Required fields in broker definitions
REQUIRED_FIELDS = {
    "broker": ["id", "name", "domain", "category"],
    "removal": ["method"],
}

# Valid values for enum-like fields
VALID_CATEGORIES = {
    "people-search",
    "background-check",
    "data-aggregator",
    "marketing",
    "social-media",
    "government-records",
    "financial",
    "other",
}

VALID_REMOVAL_METHODS = {
    "web-form",
    "email",
    "mail",
    "phone",
    "api",
    "account-required",
}

VALID_JURISDICTIONS = {
    "ccpa",
    "gdpr",
    "vcdpa",
    "cpa",
    "ctdpa",
    "ucpa",
    "global",
}


def _load_toml(filepath: Path) -> tuple[dict | None, list[str]]:
    """Load TOML file and return data or errors."""
    try:
        with open(filepath, "rb") as f:
            return tomllib.load(f), []
    except tomllib.TOMLDecodeError as e:
        return None, [f"Invalid TOML syntax: {e}"]


def _validate_required_fields(data: dict) -> list[str]:
    """Check required sections and fields are present."""
    errors = []
    for section, fields in REQUIRED_FIELDS.items():
        if section not in data:
            errors.append(f"Missing required section: [{section}]")
            continue
        for field in fields:
            if field not in data[section]:
                errors.append(f"Missing required field: {section}.{field}")
    return errors


def _validate_broker_section(broker: dict) -> list[str]:
    """Validate the broker section fields."""
    errors = []

    # Validate category
    if "category" in broker and broker["category"] not in VALID_CATEGORIES:
        errors.append(
            f"Invalid category '{broker['category']}'. "
            f"Must be one of: {', '.join(sorted(VALID_CATEGORIES))}"
        )

    # Validate domain format
    if "domain" in broker:
        domain = broker["domain"]
        if not domain or " " in domain or domain.startswith("http"):
            errors.append(
                f"Invalid domain '{domain}'. Should be just the domain (e.g., 'spokeo.com')"
            )

    # Validate ID format
    if "id" in broker:
        broker_id = broker["id"]
        if not broker_id.replace("-", "").replace("_", "").isalnum():
            errors.append(f"Invalid broker ID '{broker_id}'. Use lowercase alphanumeric with hyphens.")
        if broker_id != broker_id.lower():
            errors.append(f"Broker ID '{broker_id}' must be lowercase.")

    return errors


def _validate_removal_section(removal: dict) -> list[str]:
    """Validate the removal section fields."""
    errors = []

    # Validate method
    if "method" in removal:
        method = removal["method"]
        methods = [method] if isinstance(method, str) else method if isinstance(method, list) else []

        if not methods and "method" in removal:
            errors.append("removal.method must be a string or array of strings")

        for m in methods:
            if m not in VALID_REMOVAL_METHODS:
                errors.append(
                    f"Invalid removal method '{m}'. "
                    f"Must be one of: {', '.join(sorted(VALID_REMOVAL_METHODS))}"
                )

    # Validate URL if present
    if "url" in removal:
        url = removal["url"]
        if not url.startswith(("http://", "https://")):
            errors.append(f"removal.url must be a full URL, got '{url}'")

    return errors


def _validate_jurisdictions(jurisdictions: list) -> list[str]:
    """Validate jurisdictions section."""
    errors = []
    for jurisdiction in jurisdictions:
        if isinstance(jurisdiction, dict) and "law" in jurisdiction:
            if jurisdiction["law"] not in VALID_JURISDICTIONS:
                errors.append(
                    f"Invalid jurisdiction '{jurisdiction['law']}'. "
                    f"Must be one of: {', '.join(sorted(VALID_JURISDICTIONS))}"
                )
    return errors


def validate_broker_toml(filepath: Path) -> list[str]:
    """Validate a single broker TOML file. Returns list of errors."""
    data, errors = _load_toml(filepath)
    if not data:
        return errors

    # Check required fields
    errors.extend(_validate_required_fields(data))

    # Validate broker section
    if "broker" in data:
        errors.extend(_validate_broker_section(data["broker"]))

    # Validate removal section
    if "removal" in data:
        errors.extend(_validate_removal_section(data["removal"]))

    # Validate jurisdictions
    if "jurisdictions" in data:
        errors.extend(_validate_jurisdictions(data["jurisdictions"]))

    return errors


# Files to skip (documentation, not actual definitions)
SKIP_FILES = {"schema.toml", "README.md"}


def main() -> int:
    """Main entry point. Returns 0 on success, 1 on failure."""
    if len(sys.argv) < 2:
        print("Usage: validate-broker-toml.py <file1.toml> [file2.toml ...]")
        return 1

    all_valid = True

    for filepath in sys.argv[1:]:
        # Skip documentation files
        if Path(filepath).name in SKIP_FILES:
            print(f"SKIP: {filepath} (documentation)")
            continue
        path = Path(filepath)
        if not path.exists():
            print(f"ERROR: File not found: {filepath}")
            all_valid = False
            continue

        errors = validate_broker_toml(path)

        if errors:
            print(f"ERROR: {filepath}")
            for error in errors:
                print(f"  - {error}")
            all_valid = False
        else:
            print(f"OK: {filepath}")

    return 0 if all_valid else 1


if __name__ == "__main__":
    sys.exit(main())
