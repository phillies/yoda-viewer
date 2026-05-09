param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("fmt", "clippy", "test", "serve-web", "serve-desktop")]
    [string]$Task
)

switch ($Task) {
    "fmt" { cargo fmt --all }
    "clippy" { cargo clippy --workspace --all-targets --all-features -- -D warnings }
    "test" { cargo test --workspace }
    "serve-web" { dx serve --package yoda-web --platform web }
    "serve-desktop" { dx serve --package yoda-desktop --platform desktop }
}