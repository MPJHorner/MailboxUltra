# mac/

Hand-rolled macOS bundling. Three scripts plus an `Info.plist` template.

| File | Run with |
|---|---|
| `Info.plist` | template; `__VERSION__` is replaced from `Cargo.toml` at build time |
| `build-app.sh` | `make app` |
| `build-dmg.sh` | `make dmg` (after `make app`) |
| `build-app-universal.sh` | `make app-universal` |

The icon is generated separately by `make icon` (see `tools/icon-gen.rs`
+ `iconutil`); the result is committed at `icon/AppIcon.icns`.

## Code-signing

The scripts will sign the app and DMG iff `APPLE_CERT_ID` is set in the
environment. Without it, the bundle is unsigned and the user has to
right-click → Open the app on first launch (or run
`xattr -d com.apple.quarantine MailBoxUltra.app`).

To enable signing in CI, add `APPLE_CERT_ID` plus the matching certificate
import step (e.g. `gh secret set` for `APPLE_DEVELOPER_CERTIFICATE_P12`,
then a cert-import action) to `.github/workflows/release.yml`.

## First-launch instructions for unsigned builds

1. Drag `MailBox Ultra.app` from the DMG into `/Applications`.
2. Right-click the app → **Open**.
3. macOS will warn that the developer can't be verified. Click **Open**
   anyway. Subsequent launches just need a double-click.
