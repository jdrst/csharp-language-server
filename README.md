# roslyn-language-server
A wrapper around Roslyn language server which makes compatible with other editors, eg. Helix or Zed.

This is a work in progress, and currently only works with project containing a `.sln` project file.

If you would want to try to run it anyways, you need `Microsoft.CodeAnalysis.LanguageServer` on your path. I got it from nix package: `roslyn-ls`.

## Use with Helix
Since `Microsoft.CodeAnalysis.LanguageServer` only supports `pull diagnostics` and Helix does not (yet), you would need to use my branch at `github:sofusa/helix-pull-diagnostics`.

```toml
[language-server.roslyn]
command = "roslyn-language-server"

[[language]]
name = "c-sharp"
language-servers = ["roslyn"]
```

## To do: 
- Find and open `.csproj` files if no `.sln` file was found
