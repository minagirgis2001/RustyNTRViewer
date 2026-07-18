param(
    [Parameter(Mandatory = $true)]
    [string]$Version
)

$ErrorActionPreference = "Stop"
$Version = $Version.TrimStart("v")
$PackageName = "RustyNTRViewer-$Version-windows-x86_64"
$Dist = Join-Path $PWD "dist"
$Stage = Join-Path $Dist $PackageName

cargo build --release --locked -p rusty-ntr-viewer
New-Item -ItemType Directory -Force -Path $Stage | Out-Null
Copy-Item "target/release/rusty-ntr-viewer.exe" $Stage
Copy-Item "README.md", "LICENSE", "THIRD_PARTY_NOTICES.md" $Stage
Compress-Archive -Path $Stage -DestinationPath (Join-Path $Dist "$PackageName.zip") -Force
