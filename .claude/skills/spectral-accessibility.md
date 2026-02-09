# spectral:accessibility

Expert for accessibility and UX in Spectral. Use when auditing UI components, ensuring WCAG compliance, or improving user experience for all users.

## Expertise

You are an **Accessibility & UX Specialist** with expertise in:
- WCAG 2.1/2.2 guidelines (Level AA compliance)
- Screen reader compatibility (NVDA, VoiceOver, JAWS)
- Keyboard navigation patterns
- Color contrast and visual accessibility
- Cognitive accessibility and plain language
- Assistive technology testing

## WCAG 2.1 Level AA Requirements

### Perceivable

| Criterion | Requirement | How to Test |
|-----------|-------------|-------------|
| 1.1.1 Non-text Content | Alt text for images | Check all `<img>`, icons |
| 1.3.1 Info and Relationships | Semantic HTML | Inspect heading hierarchy, lists |
| 1.3.2 Meaningful Sequence | Logical DOM order | Tab through page |
| 1.4.1 Use of Color | Color not sole indicator | Check error states, links |
| 1.4.3 Contrast (Minimum) | 4.5:1 for text, 3:1 for large | Use contrast checker |
| 1.4.4 Resize Text | 200% zoom works | Zoom browser to 200% |
| 1.4.10 Reflow | No horizontal scroll at 320px | Resize to 320px width |
| 1.4.11 Non-text Contrast | 3:1 for UI components | Check buttons, inputs |

### Operable

| Criterion | Requirement | How to Test |
|-----------|-------------|-------------|
| 2.1.1 Keyboard | All functionality via keyboard | Tab through everything |
| 2.1.2 No Keyboard Trap | Can always Tab away | Test all modals, dropdowns |
| 2.4.1 Bypass Blocks | Skip to content link | Check for skip link |
| 2.4.2 Page Titled | Descriptive page titles | Check `<title>` |
| 2.4.3 Focus Order | Logical focus sequence | Tab through page |
| 2.4.4 Link Purpose | Links describe destination | Read link text alone |
| 2.4.6 Headings and Labels | Descriptive headings | Check heading text |
| 2.4.7 Focus Visible | Visible focus indicator | Tab and observe |

### Understandable

| Criterion | Requirement | How to Test |
|-----------|-------------|-------------|
| 3.1.1 Language of Page | `lang` attribute set | Check `<html lang="">` |
| 3.2.1 On Focus | No unexpected changes | Focus each element |
| 3.2.2 On Input | No unexpected changes | Interact with forms |
| 3.3.1 Error Identification | Errors clearly described | Submit invalid forms |
| 3.3.2 Labels or Instructions | Form inputs labeled | Check all inputs |

### Robust

| Criterion | Requirement | How to Test |
|-----------|-------------|-------------|
| 4.1.1 Parsing | Valid HTML | Run HTML validator |
| 4.1.2 Name, Role, Value | ARIA used correctly | Check with axe DevTools |

## Component Patterns

### Buttons

```svelte
<!-- Good: Accessible button -->
<button
  type="button"
  aria-label="Remove listing from Spokeo"
  aria-describedby="spokeo-status"
  disabled={isLoading}
>
  {#if isLoading}
    <span class="sr-only">Loading...</span>
    <Spinner aria-hidden="true" />
  {:else}
    Remove
  {/if}
</button>
<span id="spokeo-status" class="sr-only">Status: Found, pending removal</span>

<!-- Bad: Inaccessible -->
<div onclick={handleClick} class="button">Remove</div>
```

### Forms

```svelte
<!-- Good: Accessible form -->
<form on:submit|preventDefault={handleSubmit}>
  <div class="field">
    <label for="email">Email address</label>
    <input
      id="email"
      type="email"
      bind:value={email}
      aria-describedby="email-hint email-error"
      aria-invalid={emailError ? 'true' : undefined}
      required
    />
    <p id="email-hint" class="hint">We'll use this to verify your identity</p>
    {#if emailError}
      <p id="email-error" class="error" role="alert">{emailError}</p>
    {/if}
  </div>

  <button type="submit">Submit</button>
</form>
```

### Modals

```svelte
<!-- Good: Accessible modal -->
<div
  role="dialog"
  aria-modal="true"
  aria-labelledby="modal-title"
  aria-describedby="modal-description"
>
  <h2 id="modal-title">Confirm Removal</h2>
  <p id="modal-description">
    This will submit a removal request to Spokeo. Continue?
  </p>

  <div class="actions">
    <button type="button" on:click={close}>Cancel</button>
    <button type="button" on:click={confirm} autofocus>Confirm</button>
  </div>
</div>

<!-- Focus trap and Escape key handling required -->
<script>
  import { trapFocus } from '$lib/utils/focus-trap';
  import { onMount } from 'svelte';

  onMount(() => {
    const cleanup = trapFocus(dialogElement);
    const handleEscape = (e) => e.key === 'Escape' && close();
    window.addEventListener('keydown', handleEscape);
    return () => {
      cleanup();
      window.removeEventListener('keydown', handleEscape);
    };
  });
</script>
```

### Status Updates

```svelte
<!-- Good: Announced to screen readers -->
<div aria-live="polite" aria-atomic="true" class="sr-only">
  {statusMessage}
</div>

<!-- In component -->
<script>
  let statusMessage = '';

  async function scan() {
    statusMessage = 'Scanning Spokeo...';
    const result = await invoke('scan_broker', { id: 'spokeo' });
    statusMessage = result.found
      ? 'Found listing on Spokeo'
      : 'No listing found on Spokeo';
  }
</script>
```

### Data Tables

```svelte
<table>
  <caption class="sr-only">Broker scan results</caption>
  <thead>
    <tr>
      <th scope="col">Broker</th>
      <th scope="col">Status</th>
      <th scope="col">Actions</th>
    </tr>
  </thead>
  <tbody>
    {#each results as result}
      <tr>
        <th scope="row">{result.brokerName}</th>
        <td>
          <span class="status" aria-label={getStatusLabel(result.status)}>
            {result.status}
          </span>
        </td>
        <td>
          <button aria-label="Remove from {result.brokerName}">
            Remove
          </button>
        </td>
      </tr>
    {/each}
  </tbody>
</table>
```

## Color Contrast

### Minimum Ratios

| Element | Ratio Required |
|---------|----------------|
| Normal text (<18px) | 4.5:1 |
| Large text (≥18px or ≥14px bold) | 3:1 |
| UI components (buttons, inputs) | 3:1 |
| Focus indicators | 3:1 |

### Spectral Color Recommendations

```css
/* Ensure these pass contrast checks */
:root {
  /* Text colors - must have 4.5:1 against background */
  --text-primary: #1a1a1a;      /* On white: 16:1 */
  --text-secondary: #525252;    /* On white: 7:1 */
  --text-muted: #737373;        /* On white: 4.6:1 - barely passes */

  /* Status colors - must have 3:1 for non-text */
  --status-success: #16a34a;    /* Green - check against bg */
  --status-warning: #d97706;    /* Amber */
  --status-error: #dc2626;      /* Red */

  /* Focus indicator */
  --focus-ring: #2563eb;        /* Blue - 3:1 minimum */
}
```

## Keyboard Navigation

### Required Shortcuts

| Key | Action |
|-----|--------|
| Tab | Move to next interactive element |
| Shift+Tab | Move to previous interactive element |
| Enter/Space | Activate buttons, links |
| Escape | Close modal, dropdown, cancel |
| Arrow keys | Navigate within components (tabs, menus) |

### Focus Management

```typescript
// After dynamic content change, manage focus
function afterRemovalSubmit() {
  // Move focus to status message
  const status = document.getElementById('removal-status');
  status?.focus();
}

function afterModalClose() {
  // Return focus to trigger element
  triggerElement?.focus();
}
```

## Testing Checklist

### Automated Testing
- [ ] Run axe DevTools (browser extension)
- [ ] Run Lighthouse accessibility audit
- [ ] Run pa11y CLI on all routes
- [ ] Check HTML validity (W3C validator)

### Manual Testing
- [ ] Navigate entire app with keyboard only
- [ ] Test with screen reader (NVDA on Windows, VoiceOver on Mac)
- [ ] Test at 200% zoom
- [ ] Test at 320px width
- [ ] Test with high contrast mode
- [ ] Test with reduced motion preference

### Screen Reader Testing Script

```
1. Open app with NVDA/VoiceOver running
2. Navigate to dashboard - verify heading announced
3. Tab to broker list - verify list announced with count
4. Tab to first broker - verify name and status read
5. Activate "Scan" button - verify loading state announced
6. Wait for result - verify result announced via live region
7. Open settings modal - verify focus trapped, title announced
8. Press Escape - verify modal closes, focus returns
```

## Invocation Examples

- "Audit this component for accessibility issues"
- "Is this form WCAG 2.1 AA compliant?"
- "How should I announce this status change to screen readers?"
- "Review the keyboard navigation for the broker list"
- "Check if these colors have sufficient contrast"
