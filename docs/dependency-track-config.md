# Dependency Track Integration

## Project Information

- **Project Name:** Spectral
- **Project Version:** 0.1.0
- **Project UUID:** `693185af-a6ed-4e9f-96df-81300cd3ed93`
- **Dashboard URL:** http://192.168.1.220:8081/projects/693185af-a6ed-4e9f-96df-81300cd3ed93

## Metrics (as of 2026-02-11)

- **Total Components:** 216
- **Vulnerabilities:** 0
- **Status:** ✅ All dependencies secure

## SBOM Files

### Rust Dependencies (Cargo)
- **File:** `src-tauri/spectral-rust-sbom.json`
- **Size:** 507 KB
- **Format:** CycloneDX 1.3
- **Upload Token:** `9eb994f5-a1d6-43ab-857c-3e51cf41170c`

### JavaScript Dependencies (npm)
- **File:** `spectral-npm-sbom.json`
- **Size:** 358 KB
- **Format:** CycloneDX 1.6
- **Upload Token:** `68f65c35-4786-4ac7-ab66-adc4714bc57d`

## Generating SBOMs

### Rust/Cargo SBOM
```bash
cd src-tauri
cargo cyclonedx --format json --override-filename spectral-rust-sbom.json
```

### npm SBOM
```bash
npx @cyclonedx/cyclonedx-npm --output-file spectral-npm-sbom.json
```

## Uploading to Dependency Track

### Via API
```bash
PROJECT_UUID="693185af-a6ed-4e9f-96df-81300cd3ed93"
API_KEY="your_dependency_track_api_key_here"  # pragma: allowlist secret

# Upload Rust SBOM
curl -X POST "http://192.168.1.220:8081/api/v1/bom" \
  -H "X-Api-Key: $API_KEY" \
  -F "project=$PROJECT_UUID" \
  -F "bom=@src-tauri/spectral-rust-sbom.json"

# Upload npm SBOM
curl -X POST "http://192.168.1.220:8081/api/v1/bom" \
  -H "X-Api-Key: $API_KEY" \
  -F "project=$PROJECT_UUID" \
  -F "bom=@spectral-npm-sbom.json"
```

### Via Web UI
1. Go to http://192.168.1.220:8081
2. Navigate to Projects → Spectral
3. Click "Upload BOM"
4. Select the SBOM file (JSON format)
5. Click "Upload"

## CI/CD Integration

Add to your GitHub Actions or CI pipeline:

```yaml
- name: Generate and Upload SBOMs
  run: |
    # Generate Rust SBOM
    cd src-tauri
    cargo cyclonedx --format json --override-filename spectral-rust-sbom.json
    cd ..

    # Generate npm SBOM
    npx @cyclonedx/cyclonedx-npm --output-file spectral-npm-sbom.json

    # Upload to Dependency Track
    curl -X POST "http://192.168.1.220:8081/api/v1/bom" \
      -H "X-Api-Key: ${{ secrets.DEPENDENCY_TRACK_API_KEY }}" \
      -F "project=693185af-a6ed-4e9f-96df-81300cd3ed93" \
      -F "bom=@src-tauri/spectral-rust-sbom.json"

    curl -X POST "http://192.168.1.220:8081/api/v1/bom" \
      -H "X-Api-Key: ${{ secrets.DEPENDENCY_TRACK_API_KEY }}" \
      -F "project=693185af-a6ed-4e9f-96df-81300cd3ed93" \
      -F "bom=@spectral-npm-sbom.json"
```

## Monitoring

Check the Dependency Track dashboard regularly for:
- New vulnerabilities in dependencies
- Outdated components
- License compliance issues
- Risk score changes

## Notes

- SBOMs are automatically processed by Dependency Track after upload
- Vulnerability scanning happens automatically
- The project is set to "active" status
- Email notifications can be configured in Dependency Track settings
