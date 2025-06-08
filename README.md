# BedrockCI

[![CI](https://github.com/laurhinch/bedrockci/actions/workflows/rust.yml/badge.svg)](https://github.com/laurhinch/bedrockci/actions/workflows/rust.yml)

BedrockCI validates Minecraft Bedrock resource and behavior packs against a real server instance. Built for CI pipelines.

## Quick Start (Recommended)

The easiest way to use BedrockCI in CI pipelines is through the [GitHub Action](https://github.com/laurhinch/install-bedrockci):

```yaml
- uses: laurhinch/install-bedrockci@v1
```

See the [action's README](https://github.com/laurhinch/install-bedrockci) for complete usage examples.

## Features

- Download and manage Minecraft Bedrock server versions
- Validate resource and behavior packs against a real server instance
- CI-friendly output with configurable warning/error handling
- Linux-only support (Ubuntu recommended/required)

## Manual Installation

If you need to run BedrockCI locally or outside of GitHub Actions, download from [Releases](https://github.com/laurhinch/bedrockci/releases).

## EULA and Privacy Policy

This tool downloads official Minecraft Bedrock server directly from Microsoft. The software is not proxied or modified during download.

By using this tool, you must accept:
- [Minecraft End User License Agreement](https://minecraft.net/eula)
- [Microsoft Privacy Policy](https://go.microsoft.com/fwlink/?LinkId=521839)

The `--accept-eula` flag is required for server downloads. Without it, the download will fail.

## Usage

Do NOT modify the downloaded servers in any way, as you might break some of the logic used to time validation.

## CLI Usage

```sh
# Download specific version (requires EULA acceptance)
bedrockci download --version 1.21.84.1 --accept-eula

# Download the latest version (still requires EULA accept)
bedrockci download --accept-eula

# List installed server versions
bedrockci list

# Validate packs
bedrockci validate --rp /path/to/resource_pack --bp /path/to/behavior_pack
```

Options for `validate` command:
- `--rp`: Resource pack path (required)
- `--bp`: Behavior pack path (required)
- `--version, -v`: Server version (default: latest installed)
- `--only-warn`: Treat errors as warnings
- `--fail-on-warn`: Fail CI on warnings and errors

## Configuration

`BEDROCK_SERVER_PATH`: Server installation path (default: `~/.bedrockci/server`)

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and guidelines.

## License

MIT