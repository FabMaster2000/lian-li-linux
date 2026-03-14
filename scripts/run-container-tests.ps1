param(
    [string]$ImageName = "lianli-linux-test-runner",
    [string]$ArtifactsDir = ""
)

$ErrorActionPreference = "Stop"

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
if ([string]::IsNullOrWhiteSpace($ArtifactsDir)) {
    $ArtifactsDir = Join-Path $repoRoot "artifacts\\container-tests"
}

New-Item -ItemType Directory -Force -Path $ArtifactsDir | Out-Null

Get-Command docker -ErrorAction Stop | Out-Null

docker build -f (Join-Path $repoRoot "docker\\test-runner.Dockerfile") -t $ImageName $repoRoot
if ($LASTEXITCODE -ne 0) {
    throw "docker build failed with exit code $LASTEXITCODE"
}

docker run --rm -t `
    -v "${repoRoot}:/work" `
    -v "${ArtifactsDir}:/artifacts" `
    $ImageName `
    bash /work/scripts/run-test-suite.sh /artifacts

if ($LASTEXITCODE -ne 0) {
    throw "container test run failed with exit code $LASTEXITCODE"
}

$reportPath = Join-Path $ArtifactsDir "container-tests-report.md"
if (Test-Path $reportPath) {
    Write-Host "Combined report written to $reportPath"
}
else {
    throw "container test report was not generated: $reportPath"
}
