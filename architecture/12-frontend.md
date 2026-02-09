## 12. Frontend Architecture

### 12.1 Views

**Dashboard** — Primary landing page
- Privacy score with grade badge and trend chart
- Scan coverage: X of Y known brokers scanned
- Active removals in progress with legal deadlines and timeline
- Network activity summary (broker contacts, tracker contacts, trend vs. baseline)
- Local PII findings summary (critical/medium/informational counts)
- Communication threads status (awaiting broker, needs your reply)
- Recent activity feed
- "Quick scan" and "Full scan" action buttons

**Chat** — Conversational interface (full-screen or slide-out panel)
- Message history with tool-use indicators (shows when Spectral is scanning, submitting, etc.)
- Suggested actions as quick-reply chips
- Inline status cards for scan results and removal progress
- Markdown rendering for detailed explanations

**Broker Explorer** — Browse and search the broker database
- Filterable table/grid of all known brokers
- Category filters, difficulty ratings, status per broker
- Broker detail pages with opt-out procedure documentation
- Community contribution UI (suggest edits, flag broken procedures)
- Network telemetry overlay (seen in your traffic? yes/no)

**Local Discovery** — PII findings across your filesystem, email, and browser
- Grouped by source (file, email, browser) and risk level
- Recommended actions with one-click remediation
- Scan scheduling and scope configuration

**Profile** — Manage your PII
- Add/edit personal information used for scanning
- Clear visual indicators of what data is stored
- Field-level encryption status visible
- Export/delete all data

**Settings**
- LLM provider configuration (API keys for cloud, model selection for local)
- Scan scheduling (frequency, time of day)
- Permission management (preset selector + fine-grained overrides)
- Privacy Audit Log viewer
- Proxy/VPN configuration
- Notification preferences
- Plugin management
- Security settings (auto-lock timeout, Argon2 parameters)

### 12.2 Tauri IPC Design

All communication between frontend and Rust backend goes through strongly-typed Tauri commands:

```rust
// src-tauri/src/commands/vault.rs
#[tauri::command]
async fn unlock_vault(password: String, state: State<'_, AppState>) -> Result<bool, String>;

#[tauri::command]
async fn get_dashboard_summary(state: State<'_, AppState>) -> Result<DashboardSummary, String>;

// src-tauri/src/commands/scan.rs
#[tauri::command]
async fn start_scan(
    broker_ids: Option<Vec<String>>,
    state: State<'_, AppState>,
) -> Result<ScanJobId, String>;

#[tauri::command]
async fn get_scan_progress(job_id: ScanJobId, state: State<'_, AppState>) -> Result<ScanProgress, String>;

// src-tauri/src/commands/chat.rs
#[tauri::command]
async fn send_message(
    message: String,
    state: State<'_, AppState>,
) -> Result<ChatResponse, String>;

// src-tauri/src/commands/permissions.rs
#[tauri::command]
async fn get_permission_status(state: State<'_, AppState>) -> Result<PermissionSummary, String>;

#[tauri::command]
async fn respond_to_permission_prompt(
    prompt_id: Uuid,
    decision: PermissionDecision,
    state: State<'_, AppState>,
) -> Result<(), String>;

// src-tauri/src/commands/netmon.rs
#[tauri::command]
async fn get_privacy_score(state: State<'_, AppState>) -> Result<PrivacyScore, String>;

#[tauri::command]
async fn get_network_summary(
    period: ReportPeriod,
    state: State<'_, AppState>,
) -> Result<NetworkSummary, String>;
```

Streaming responses (for LLM chat and scan progress) use Tauri's event system:

```rust
app_handle.emit("chat:stream", StreamChunk { text: "..." })?;
app_handle.emit("scan:progress", ScanProgressEvent { ... })?;
app_handle.emit("permission:request", PermissionPrompt { ... })?;
app_handle.emit("netmon:alert", NetmonAlert { ... })?;
app_handle.emit("verify:status_change", VerificationEvent { ... })?;
```

### 12.3 UI Adaptation for LLM-Optional Mode

The frontend dynamically adapts based on the capability registry:

```typescript
// /src/hooks/useCapabilities.ts

interface Capabilities {
  llmEnabled: boolean;
  features: Record<FeatureId, FeatureConfig>;
}

// Components conditionally render based on capabilities
function MainLayout() {
  const caps = useCapabilities();

  return (
    <AppShell>
      <Sidebar>
        {caps.llmEnabled && caps.features.ChatInterface?.enabled
          ? <ChatNavItem />
          : <CommandPaletteNavItem />
        }
        <DashboardNavItem />
        <BrokerExplorerNavItem />
        {caps.features.FileSystemPiiScan?.enabled && <LocalDiscoveryNavItem />}
        <ProfileNavItem />
        <SettingsNavItem />
      </Sidebar>
      <MainContent />
    </AppShell>
  );
}
```

When LLM is disabled, the Chat panel is replaced with a **Command Palette** — a structured interface with:
- Dropdown menus for actions (Scan, Remove, Check Status, etc.)
- Wizard-style flows for multi-step operations
- Search/filter for broker database
- Tabular status views with sortable columns

This ensures the app is fully functional and still user-friendly without any AI dependency.

---
