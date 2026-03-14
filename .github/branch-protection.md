# Branch Protection Rules — `main`

Configure these settings via **GitHub → Settings → Branches → Add rule → `main`**.

## Required Settings

| Setting | Value |
|---|---|
| Require pull request before merging | ✅ enabled |
| Required approving reviews | 1 |
| Dismiss stale pull request approvals when new commits are pushed | ✅ enabled |
| Require status checks to pass before merging | ✅ enabled |
| Required status checks | `fmt`, `clippy`, `test-linux` |
| Require branches to be up to date before merging | ✅ enabled |
| Allow force pushes | ❌ disabled |
| Allow deletions | ❌ disabled |

## Step-by-step Setup

1. Go to **Settings → Branches** in the GitHub repository.
2. Click **Add branch protection rule**.
3. In **Branch name pattern** enter: `main`
4. Enable **Require a pull request before merging**:
   - Set **Required approving reviews** to `1`
   - Check **Dismiss stale pull request approvals when new commits are pushed**
5. Enable **Require status checks to pass before merging**:
   - Check **Require branches to be up to date before merging**
   - In the search box, add these status checks (must run CI at least once to appear):
     - `fmt`
     - `clippy`
     - `test-linux`
6. Disable **Allow force pushes** (should be off by default).
7. Disable **Allow deletions** (should be off by default).
8. Click **Create** (or **Save changes**).

## Why These Checks

| Check | Reason |
|---|---|
| `fmt` | Enforces `cargo fmt --all` before every merge |
| `clippy` | Zero clippy warnings policy (`-D warnings`) |
| `test-linux` | Core logic tests on Linux (jarvis-core + jarvis-cli) |

`test-windows` is NOT required (Windows runner is slower and used mainly for release builds).

## Notes

- After merging Phase D and running CI once, status check names will appear in the search dropdown.
- If the repository is private, GitHub Actions minutes are shared (2000/month free tier).
- For public repositories, GitHub Actions are unlimited.
