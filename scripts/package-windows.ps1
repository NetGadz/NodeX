# Package Windows distribution
param(
    [string]$Version = "1.0.0"
)

& "$PSScriptRoot\..\deploy\windows\package.ps1" -Version $Version
