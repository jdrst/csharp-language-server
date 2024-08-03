# roslyn-language-server
A wrapper around the language server behind the C# Visual Studio Code extension, `Microsoft.CodeAnalysis.LanguageServer`, which makes it compatible with other editors, e.g., Helix.
This is more stable and faster than OmniSharp.

This has only been tested on Linux. 

This tool works around the quirks of `Microsoft.CodeAnalysis.LanguageServer` in the following way: 
- Launches `Microsoft.CodeAnalysis.LanguageServer` as a process
- Passes the provided `unix socket` and forwards all communication to `stdio`
- Waits for `Capabilities` notification from server
  - Forces `pull diagnostics` to be available. This is a hack to make the server respect clients who does not support dynamic regisration of diagnostic capabilities. This is should be considered a bug in the server and can hopefully be removed with a future version of server
- Waits for an `Initialize` notification from the client
  - Finds relevant `.sln` or `.csproj` files and sends them to the server as an `open` notification.

# Installation

## `Microsoft.CodeAnalysis.LanguageServer`
The wrapper uses `Microsoft.CodeAnalysis.LanguageServer` so you need this on your path. 
If you use `nix`, you can grab `nixpkgs.roslyn-ls`. 

Otherwise:
- Find and download `Microsoft.CodeAnalysis.LanguageServer` for your architecture at the [public feed](https://dev.azure.com/azure-public/vside/_artifacts/feed/vs-impl).
- Unzip the `.nupkg` file with `unzip`
- Find and move the `Microsoft.CodeAnalysis.LanguageServer` executable to a directory on your path, e.g., `~/.local/bin`.

## The wrapper
If you use `nix`, you can use this repository's `nix flake`. 

Alternatively, install with `cargo`: `cargo install --git https://github.com/SofusA/roslyn-language-server` 

## Use with Helix
Since `Microsoft.CodeAnalysis.LanguageServer` only supports `pull diagnostics` and Helix does not (yet), you would need to use my branch at `github:sofusa/helix-pull-diagnostics`.

```toml
[language-server.roslyn]
command = "roslyn-language-server"

[[language]]
name = "c-sharp"
language-servers = ["roslyn"]
```
