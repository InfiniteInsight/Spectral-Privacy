# Spectral — Open-Source Personal Data Removal Platform

## Unified Architecture Document v0.3

> **v0.3 changes:** Added user onboarding wizard (Section 20), geolocation/jurisdiction system (Section 21), proactive broker scanning model (Section 22), commercial relationship engine for non-data-broker deletion (Section 23), and resolved all 10 open questions (Section 24). Section 19 is now historical reference only.

---

## Table of Contents

1. [Vision & Problem Statement](#1-vision--problem-statement)
2. [High-Level Architecture](#2-high-level-architecture)
3. [Core Modules — Detailed Design](#3-core-modules--detailed-design)
   - 3.1 Encrypted Vault
   - 3.2 LLM Router & Adapter
   - 3.3 LLM-Optional Architecture
   - 3.4 Broker Engine
   - 3.5 Browser Automation
   - 3.6 Plugin System
   - 3.7 Conversational Interface
4. [Local PII Discovery Engine](#4-local-pii-discovery-engine)
   - 4.1 Discovery Architecture
   - 4.2 Core Types
   - 4.3 Filesystem Scanner
   - 4.4 Email Scanner
   - 4.5 Browser Data Scanner
5. [Network Telemetry Engine](#5-network-telemetry-engine)
   - 5.1 Architecture Overview
   - 5.2 Data Source Adapters
   - 5.3 Platform-Specific Collectors
   - 5.4 Domain Intelligence Database
   - 5.5 Collection Scheduling & Baseline Building
   - 5.6 Baseline & Trend Database
   - 5.7 Privacy Score Calculation
6. [Removal Verification & Follow-Up Engine](#6-removal-verification--follow-up-engine)
   - 6.1 Verification Pipeline
   - 6.2 Core Types
   - 6.3 Legal Timeline Tracking
7. [Third-Party Communication Engine](#7-third-party-communication-engine)
   - 7.1 Overview & Threat Model
   - 7.2 Communication State Machine
   - 7.3 Core Types
   - 7.4 LLM Safety Guardrails for Third-Party Content
   - 7.5 Auto-Reply Budget & Limits
   - 7.6 Static Reply Templates
8. [Granular Permission System](#8-granular-permission-system)
   - 8.1 Permission Architecture
   - 8.2 Permission Request Flow
   - 8.3 Permission Presets & First-Run Wizard
   - 8.4 Audit & Transparency System
9. [Security Architecture](#9-security-architecture)
   - 9.1 Threat Model
   - 9.2 PII Handling Rules
   - 9.3 Authentication & Key Management
10. [Reporting & Progress Dashboard](#10-reporting--progress-dashboard)
    - 10.1 Report Types
    - 10.2 Dashboard Widgets
    - 10.3 Cross-Correlation Intelligence
11. [LLM Integration Strategy](#11-llm-integration-strategy)
    - 11.1 Task Classification & Routing
    - 11.2 Local LLM Recommendations
    - 11.3 Feature Behavior: LLM On vs. Off
    - 11.4 Configuration
12. [Frontend Architecture](#12-frontend-architecture)
    - 12.1 Views
    - 12.2 Tauri IPC Design
    - 12.3 UI Adaptation for LLM-Optional Mode
13. [Project Structure](#13-project-structure)
14. [Database Schema](#14-database-schema)
15. [Dependencies](#15-dependencies)
16. [Broker Database Maintenance](#16-broker-database-maintenance)
17. [Development Roadmap](#17-development-roadmap)
18. [License Recommendation](#18-license-recommendation)
19. [Open Questions & Discussion Points](#19-open-questions--discussion-points) *(resolved — see Section 24)*
20. [User Onboarding & PII Profile Setup](#20-user-onboarding--pii-profile-setup)
21. [Geolocation & Jurisdiction System](#21-geolocation--jurisdiction-system)
22. [Proactive Broker Scanning Model](#22-proactive-broker-scanning-model)
23. [Commercial Relationship Engine (Non-Data-Broker Deletion)](#23-commercial-relationship-engine-non-data-broker-deletion)
24. [Resolved Open Questions](#24-resolved-open-questions)

---
