# roslyn-language-server
A wrapper around Roslyn language server which makes compatible with other editors, eg. Helix or Zed.

This is very much work in progres and does not work!
If you would want to try to run it anyways, you need `Microsoft.CodeAnalysis.LanguageServer` on your path. I got it from nix package: `roslyn-ls` 

# (Planned) Features
- Use stdin
- Find solution or project files automatically
- Cli tool to download and upgrade Roslyn Language Server (The dll provided by dotnet)
