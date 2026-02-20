# Privacy & LLM Integration - Manual Testing Checklist

This checklist covers manual testing scenarios for the privacy engine and LLM integration features.

## Prerequisites

- Spectral application built and running
- At least one local LLM provider configured (Ollama or LM Studio)
- API keys available for cloud providers (optional, for cloud testing)

## Privacy Level Presets

### Test 1: Paranoid Mode
- [ ] Navigate to Settings > Privacy & Security
- [ ] Select "Paranoid" privacy level
- [ ] Verify all features are disabled in feature flags list
- [ ] Attempt to draft an email
- [ ] Expected: Operation should be blocked with privacy error message
- [ ] Attempt to fill a form
- [ ] Expected: Operation should be blocked with privacy error message

### Test 2: Local Privacy Mode
- [ ] Select "Local Privacy" privacy level
- [ ] Verify cloud LLM is disabled, local LLM is enabled
- [ ] Verify automation features are enabled
- [ ] Set LM Studio or Ollama as primary provider
- [ ] Draft an email using local LLM
- [ ] Expected: Email draft succeeds using local provider
- [ ] Attempt to use cloud provider (OpenAI/Claude)
- [ ] Expected: Operation blocked with privacy message

### Test 3: Balanced Mode (Default)
- [ ] Select "Balanced" privacy level
- [ ] Verify both local and cloud LLMs are enabled
- [ ] Verify PII scanning is enabled
- [ ] Draft email with PII (email addresses, phone numbers)
- [ ] Expected: PII should be filtered/tokenized before cloud LLM
- [ ] Verify response contains original PII (detokenized)
- [ ] Draft email without PII
- [ ] Expected: Normal processing, no filtering needed

### Test 4: Custom Mode
- [ ] Select "Custom" privacy level
- [ ] Toggle individual feature flags
- [ ] Disable "Allow Cloud LLM", enable "Allow Local LLM"
- [ ] Draft email
- [ ] Expected: Should use local provider only
- [ ] Enable all features
- [ ] Expected: All operations allowed

## LLM Provider Configuration

### Test 5: Primary Provider Selection
- [ ] Navigate to Settings > LLM Providers
- [ ] Select "Ollama" as primary provider
- [ ] Draft email without task-specific override
- [ ] Expected: Uses Ollama
- [ ] Change primary to "Claude"
- [ ] Set privacy to "Balanced"
- [ ] Add Claude API key
- [ ] Draft email
- [ ] Expected: Uses Claude (if API key valid)

### Test 6: Task-Specific Provider Override
- [ ] Set primary provider to "Ollama"
- [ ] Set "Email Draft" task to use "OpenAI"
- [ ] Set privacy to "Balanced"
- [ ] Add OpenAI API key
- [ ] Draft email
- [ ] Expected: Uses OpenAI despite Ollama being primary
- [ ] Fill form
- [ ] Expected: Uses Ollama (no override for form fill)

### Test 7: API Key Management
- [ ] Navigate to LLM Providers settings
- [ ] Add API key for OpenAI
- [ ] Click "Test Connection" (if implemented)
- [ ] Expected: Key is stored securely, connection works
- [ ] Update API key to invalid value
- [ ] Expected: Error message when attempting to use provider
- [ ] Delete API key
- [ ] Expected: Provider no longer available for selection

### Test 8: Local Provider Configuration
- [ ] Select Ollama as provider
- [ ] Verify no API key required
- [ ] Test connection to http://localhost:11434
- [ ] Expected: Can list available models
- [ ] Select LM Studio
- [ ] Test connection to http://localhost:1234
- [ ] Expected: Can list available models

## Email Drafting

### Test 9: Email Draft with Cloud Provider
- [ ] Set privacy to "Balanced"
- [ ] Set email task provider to "Claude"
- [ ] Add valid Claude API key
- [ ] Open email draft interface
- [ ] Enter: "Draft a professional email to john@example.com requesting a meeting"
- [ ] Expected: Draft generated successfully
- [ ] Verify PII (email) was filtered during processing
- [ ] Verify final output contains original email address

### Test 10: Email Draft with Local Provider
- [ ] Set email task provider to "Ollama"
- [ ] Ensure Ollama is running locally
- [ ] Enter: "Write a casual email about the project status"
- [ ] Expected: Draft generated using local LLM
- [ ] Verify no network requests to cloud services
- [ ] Compare quality/speed with cloud provider

## Form Filling

### Test 11: Form Fill with Privacy Constraints
- [ ] Set privacy to "Local Privacy"
- [ ] Navigate to form filling interface
- [ ] Load a form with PII fields (name, email, phone)
- [ ] Request auto-fill
- [ ] Expected: Uses local LLM only
- [ ] Verify form filled with appropriate values
- [ ] Verify no data sent to cloud

## Error Handling

### Test 12: Missing API Key
- [ ] Set provider to "OpenAI"
- [ ] Delete OpenAI API key
- [ ] Attempt email draft
- [ ] Expected: Clear error message about missing API key
- [ ] Expected: Suggestion to add key or switch to local provider

### Test 13: Provider Unavailable
- [ ] Set provider to "Ollama"
- [ ] Stop Ollama service
- [ ] Attempt email draft
- [ ] Expected: Connection error with troubleshooting info
- [ ] Expected: Suggestion to start service or use alternative

### Test 14: Privacy Policy Violation
- [ ] Set privacy to "Paranoid"
- [ ] Force attempt to use cloud LLM (via API/debug)
- [ ] Expected: Request blocked at privacy engine level
- [ ] Expected: Logged security event (if audit logging enabled)

## Edge Cases

### Test 15: Switching Privacy Levels Mid-Task
- [ ] Start email draft with "Balanced" mode
- [ ] Switch to "Paranoid" during drafting
- [ ] Expected: In-progress operation either completes or fails gracefully
- [ ] New operations blocked per Paranoid settings

### Test 16: Provider Fallback
- [ ] Set primary to unavailable provider
- [ ] Expected: System suggests fallback to available provider
- [ ] Or clearly indicates no providers available

### Test 17: Large PII-Heavy Content
- [ ] Set provider to cloud with PII filtering
- [ ] Enter content with 50+ email addresses and phone numbers
- [ ] Expected: All PII correctly tokenized and detokenized
- [ ] Verify performance acceptable (<5s for processing)

## Performance

### Test 18: Response Time - Local Provider
- [ ] Use Ollama with small model (7B parameters)
- [ ] Draft email with 50-word prompt
- [ ] Measure response time
- [ ] Expected: <10 seconds for completion

### Test 19: Response Time - Cloud Provider
- [ ] Use Claude/GPT-4 with PII filtering
- [ ] Draft email with 50-word prompt containing PII
- [ ] Measure response time
- [ ] Expected: <5 seconds including filtering overhead

## Accessibility

### Test 20: Screen Reader Navigation
- [ ] Enable screen reader (NVDA/JAWS/VoiceOver)
- [ ] Navigate Settings > Privacy & Security
- [ ] Expected: All controls properly labeled
- [ ] Expected: Privacy level changes announced
- [ ] Expected: Error messages readable

### Test 21: Keyboard Navigation
- [ ] Navigate privacy settings using Tab/Arrow keys only
- [ ] Select privacy levels with keyboard
- [ ] Toggle feature flags with keyboard
- [ ] Expected: All interactive elements reachable
- [ ] Expected: Clear focus indicators
- [ ] Expected: Consistent navigation order

## Sign-Off

**Tester Name:** _______________
**Date:** _______________
**Build Version:** _______________
**Test Result:** Pass / Fail (circle one)

**Notes:**
