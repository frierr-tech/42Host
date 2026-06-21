param(
    [Parameter(Mandatory = $true)]
    [string] $InstallDir
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

$runtimeDir = Join-Path $InstallDir 'runtime'
$javaExe = Join-Path $runtimeDir 'bin\java.exe'
$downloadUrl = 'https://api.adoptium.net/v3/binary/latest/21/ga/windows/x64/jre/hotspot/normal/eclipse'

if (-not (Test-Path -LiteralPath $javaExe -PathType Leaf)) {
    $tempDir = Join-Path ([IO.Path]::GetTempPath()) ("42host-java-" + [guid]::NewGuid().ToString('N'))
    $archive = Join-Path $tempDir 'java.zip'
    $expanded = Join-Path $tempDir 'expanded'
    New-Item -ItemType Directory -Path $expanded -Force | Out-Null

    try {
        $downloaded = $false
        for ($attempt = 1; $attempt -le 3 -and -not $downloaded; $attempt++) {
            try {
                Invoke-WebRequest -UseBasicParsing -Uri $downloadUrl -OutFile $archive -MaximumRedirection 10
                $downloaded = $true
            }
            catch {
                if ($attempt -eq 3) { throw }
                Start-Sleep -Seconds (2 * $attempt)
            }
        }

        Expand-Archive -LiteralPath $archive -DestinationPath $expanded -Force
        $downloadedJava = Get-ChildItem -LiteralPath $expanded -Filter java.exe -File -Recurse |
            Where-Object { $_.Directory.Name -eq 'bin' } |
            Select-Object -First 1

        if ($null -eq $downloadedJava) {
            throw 'Java runtime archive does not contain bin\java.exe'
        }

        $runtimeRoot = Split-Path -Parent (Split-Path -Parent $downloadedJava.FullName)
        if (Test-Path -LiteralPath $runtimeDir) {
            Remove-Item -LiteralPath $runtimeDir -Recurse -Force
        }
        Move-Item -LiteralPath $runtimeRoot -Destination $runtimeDir
    }
    finally {
        if (Test-Path -LiteralPath $tempDir) {
            Remove-Item -LiteralPath $tempDir -Recurse -Force
        }
    }
}

if (-not (Test-Path -LiteralPath $javaExe -PathType Leaf)) {
    throw 'Java 21 installation failed'
}

# Point new servers at the private runtime without overwriting a custom Java path.
$configDir = Join-Path $env:APPDATA '42host'
$settingsFile = Join-Path $configDir 'settings.json'
New-Item -ItemType Directory -Path $configDir -Force | Out-Null

if (Test-Path -LiteralPath $settingsFile) {
    $settings = Get-Content -LiteralPath $settingsFile -Raw | ConvertFrom-Json
}
else {
    $settings = [pscustomobject]@{}
}

$currentJava = $settings.java_path
if ([string]::IsNullOrWhiteSpace($currentJava) -or $currentJava -in @('java', 'java.exe')) {
    $settings | Add-Member -NotePropertyName java_path -NotePropertyValue $javaExe -Force
    $settingsJson = ConvertTo-Json -InputObject $settings -Depth 20
    [IO.File]::WriteAllText($settingsFile, $settingsJson, [Text.UTF8Encoding]::new($false))
}

$serversFile = Join-Path $configDir 'servers.json'
if (Test-Path -LiteralPath $serversFile) {
    $servers = @(Get-Content -LiteralPath $serversFile -Raw | ConvertFrom-Json)
    $changed = $false
    foreach ($server in $servers) {
        if ([string]::IsNullOrWhiteSpace($server.java_path) -or $server.java_path -in @('java', 'java.exe')) {
            $server | Add-Member -NotePropertyName java_path -NotePropertyValue $javaExe -Force
            $changed = $true
        }
    }
    if ($changed) {
        $serversJson = ConvertTo-Json -InputObject @($servers) -Depth 20
        [IO.File]::WriteAllText($serversFile, $serversJson, [Text.UTF8Encoding]::new($false))
    }
}
