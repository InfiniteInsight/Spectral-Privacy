## 19. Open Questions & Discussion Points

> **All 10 questions below have been resolved.** See **Section 24** for binding architectural decisions. This section is retained for historical context.

1. ~~**Email sending**~~ → **Resolved in Section 24.1.** Both draft and SMTP modes, user chooses during onboarding.

2. ~~**CAPTCHA handling**~~ → **Resolved in Section 24.2.** Pause and present to user. No automated solving.

3. ~~**Verification email handling**~~ → **Resolved in Section 24.3.** User choice: manual, or auto-click with domain-matching safety rules.

4. ~~**Legal compliance**~~ → **Resolved in Section 24.4.** Disclaimer included, no tool-assisted disclosure in submissions.

5. ~~**Telemetry-free analytics**~~ → **Resolved in Section 24.5.** Community reporting + CI pipeline, no telemetry.

6. ~~**Name**~~ → **Resolved in Section 24.6.** "Spectral" retained. "Privacy Shroud" noted as alternative.

7. ~~**Network monitoring permissions**~~ → **Resolved in Section 24.7.** Graceful degradation, no elevation required.

8. ~~**Domain intelligence false positives**~~ → **Resolved in Section 24.8.** Local whitelist + community PRs.

9. ~~**Auto-reply scope creep**~~ → **Resolved in Section 24.9.** Global daily cap of 10, hourly cap of 3, configurable.

10. ~~**Legal escalation templates**~~ → **Resolved in Section 24.10.** Disclaimer required, user chooses app-assisted or DIY.

---
