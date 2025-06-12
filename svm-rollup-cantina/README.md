# SVM Rollup
This repository contains the code to spin up a Solana Virtual Machine (SVM) rollup with the Sovereign SDK.
For additional information regarding the motivation and design, see the [tech spec](https://www.notion.so/9len/DRAFT-Tech-Spec-Sovereign-SDK-SVM-d84c41ba6c2645c9927c8f61c09565a0).

*Note that this code has not yet been audited and is still in development.

## Setup
Follow the instructions in `crates/rollup`'s README. Install the Cargo helpers, including zkVM toolchains:
```
just install-dev-tools
```

## Compiling + Testing
To compile:
```shell
just build
```
To run comprehensive tests, use:
```shell
just test
```
Both of the above commands disable Risc0 to speed up build times.
To disable either Risc0 or SP1 for all commands, run `export SKIP_RISC0_GUEST_BUILD=1` or `export SKIP_SP1_GUEST_BUILD=1`. 
To re-enable them, remove the variables from the environment via `unset <VAR_NAME>`.
Note that the build code merely checks for the presence of these variables, so setting them to an empty string will still skip builds.

## Debugging
### No Automatic CI Run
This is likely because the YAML is malformed. Open the "Actions" tab to see the problematic line.

### CI Failures

While investigating CI failures, it may be useful to add telemetry to a job. The [`workflow-telemetry`](https://github.com/marketplace/actions/workflow-telemetry) action is useful to track metrics like CPU, memory, and disk IO. To incorporate into the workflow, set up the requisite environment variables and permissions: 
```yaml
env:
   GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}

permissions:
  actions: read
  contents: read
  pull-requests: write
```
Then add `workflow-telemetry` as a step before the job whose metrics should be recorded.
```yaml
   - name: Collect Workflow Telemetry
     uses: catchpoint/workflow-telemetry-action@v2
```
The action will create graphs on the pull request directly.

### Incremental Compilation Error

The incremental compilation error (ICE) with `evaluate_obligation` is considered a bug and was reported on `rust-lang` issues by many users: see the [open issues](https://github.com/rust-lang/rust/issues?q=is%3Aissue+is%3Aopen+evaluate_obligation). The error message can appear when running tests several times in a row and will contain the following messages:

```
 error: the compiler unexpectedly panicked. this is a bug.

  note: we would appreciate a bug report: https://github.com/rust-lang/rust/issues/new?labels=C-bug%2C+I-ICE%2C+T-compiler&template=ice.md

  note: rustc 1.79.0 (129f3b996 2024-06-10) running on aarch64-apple-darwin

  note: compiler flags: --crate-type lib -C embed-bitcode=no -C debuginfo=2 -C split-debuginfo=unpacked -C incremental=[REDACTED]

  note: some of the compiler flags provided by cargo are hidden
```
```
 there was a panic while trying to force a dep node
  try_mark_green dep node stack:
  #0 exported_symbols(svm_rollup[9218])
  end of try_mark_green dep node stack
  error: could not compile `svm-rollup` (lib) due to 2 previous errors
  warning: build failed, waiting for other jobs to finish...
  error: command `/Users/<your_username>/.rustup/toolchains/1.79-aarch64-apple-darwin/bin/cargo test --no-run --message-format json-render-diagnostics --workspace --all-features` exited with code 101
  just: *** [test] Error 101
```

The temporary solution for this issues is to remove the incremental cache before running tests on your local machine again.

```shell
$ cargo clean -p svm-rollup
# or
$ cargo clean
```

## Running in cloud

In order to run the rollup on a cloud instance, we should follow these steps:

1. Once connected to the instance, we need to authorize `git` to our private repositories. 
An easy way to do this is with `gh` ([GitHub CLI](https://github.com/cli/cli/blob/trunk/docs/install_linux.md#debian-ubuntu-linux-raspberry-pi-os-apt)) which has the following installation script:
```bash
(type -p wget >/dev/null || (sudo apt update && sudo apt-get install wget -y)) \
	&& sudo mkdir -p -m 755 /etc/apt/keyrings \
	&& wget -qO- https://cli.github.com/packages/githubcli-archive-keyring.gpg | sudo tee /etc/apt/keyrings/githubcli-archive-keyring.gpg > /dev/null \
	&& sudo chmod go+r /etc/apt/keyrings/githubcli-archive-keyring.gpg \
	&& echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" | sudo tee /etc/apt/sources.list.d/github-cli.list > /dev/null \
	&& sudo apt update \
	&& sudo apt install gh -y
```
Then login using:
```bash
echo "<YOUR-TOKEN>" | gh auth login -p https --with-token
```
**NOTE:** You can create a github token for your account at [https://github.com/settings/tokens], you need to have `repo`, `workflow` and `read:org` permissions at least.

2. Clone the repository:
```bash
git clone https://github.com/nitro-svm/svm-rollup
```

3. Go into directory:
```bash
cd svm-rollup
```

4. Run environment setup script:
```bash
./scripts/debian_setup.sh
```

5. Run tooling installation:
```bash
just install-dev-tools
```

Now you are ready to compile and run the rollup.
