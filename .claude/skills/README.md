# Spectral Custom Skills

This directory contains specialized skill definitions for Spectral development.

## Primary Orchestrator

| Skill | Role |
|-------|------|
| **`spectral:pm`** | Project Manager - coordinates all other skills, ensures quality gates, manages workflows |

**Use `spectral:pm` as the default for complex tasks.** It will activate specialists as needed.

## Specialist Skills

| Skill | Expertise | Use When |
|-------|-----------|----------|
| `spectral:patterns` | Code quality, pattern compliance | Reviewing code, implementing features |
| `spectral:infosec` | Application security | Security reviews, threat modeling |
| `spectral:pentester` | Offensive security | Testing security controls, finding vulnerabilities |
| `spectral:broker-research` | Data broker landscape | Researching new brokers, opt-out procedures |
| `spectral:broker-definition` | TOML configuration | Creating/updating broker definitions |
| `spectral:legal` | Privacy law expertise | Jurisdiction rules, legal templates |
| `spectral:email-safety` | Email communication security | Email templates, prompt injection defense |
| `spectral:accessibility` | WCAG compliance, UX | Auditing UI, screen reader support |

## How to Use

### In Conversation

Reference the skill to load its expertise:

```
Using spectral:patterns, review this code for compliance...

Using spectral:legal, what laws apply to a CA resident...

Using spectral:broker-definition, create a definition for FastPeopleSearch...
```

### With Subagents

When spawning Task agents, include skill context:

```
Task prompt: "Using the spectral:infosec skill, perform a security review of the vault encryption implementation in crates/spectral-vault/"
```

### Skill Combinations

Some tasks benefit from multiple skills:

| Task | Skills |
|------|--------|
| New broker integration | `broker-research` → `broker-definition` → `patterns` |
| Email feature review | `email-safety` + `infosec` + `patterns` |
| UI component audit | `accessibility` + `patterns` |
| Legal template creation | `legal` + `email-safety` |

## Skill Development

To add a new skill:

1. Create `skill-name.md` in this directory
2. Include sections:
   - Expertise description
   - Relevant knowledge/checklists
   - Output formats
   - Invocation examples
3. Update this README

## Quick Reference

### Code Quality
- `spectral:patterns` - Pattern enforcement, code review

### Security
- `spectral:infosec` - Defensive security review
- `spectral:pentester` - Offensive security testing
- `spectral:email-safety` - Email/LLM security

### Domain Expertise
- `spectral:broker-research` - Data broker research
- `spectral:broker-definition` - Broker TOML files
- `spectral:legal` - Privacy law, jurisdiction

### User Experience
- `spectral:accessibility` - WCAG, screen readers
