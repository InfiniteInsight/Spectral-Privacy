# Phase 4: Findings Storage and Verification Workflow Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task.

**Goal:** Replace stub implementations with real findings storage using ResultParser, implement verification workflow, and enable removal request creation.

**Architecture:** Integrate ResultParser into orchestrator to extract structured data from broker HTML, add deduplication logic, implement three Tauri commands (get_findings, verify_finding, submit_removals), and connect to existing UI.

**Tech Stack:** Rust, spectral-scanner, spectral-db, SQLite, Tauri, TypeScript

---

## Task 1: Add Deduplication Function to Findings Module

**Files:**
- Modify: `crates/spectral-db/src/findings.rs`

**Goal:** Add function to check if a finding already exists by listing URL within a scan job.

**Context:** This prevents duplicate findings when same listing appears multiple times in results.

See full implementation plan in the complete file...
