param(
  [string]$PlanPath = "PLAN.md",
  [string]$LogPath = "RUNLOG.md",
  [string]$LastMessagePath = "LAST_RESPONSE.md",
  [string]$DecisionMessagePath = "DECISION_RESPONSE.md",
  [string]$Workspace = (Get-Location).Path,
  [int]$Iterations = 1,
  [int]$DelaySeconds = 0,
  [int]$DecisionRounds = 3,
  [string]$RepairMessagePath = "REPAIR_RESPONSE.md",
  [int]$RepairRounds = 2,
  [int]$RepairDelaySeconds = 0,
  [bool]$AutoRepair = $true,
  [bool]$ContinueOnError = $false,
  [switch]$Forever,
  [switch]$FullAuto,
  [string]$Profile = "",
  [bool]$AutoDecide = $true
)

# Force UTF-8 for consistent console/log encoding
try { $OutputEncoding = [System.Text.Encoding]::UTF8 } catch {}
try { [Console]::OutputEncoding = [System.Text.Encoding]::UTF8 } catch {}
try { [Console]::InputEncoding = [System.Text.Encoding]::UTF8 } catch {}

function Resolve-WorkspacePath {
  param([string]$Path)
  if ([string]::IsNullOrWhiteSpace($Path)) {
    throw "Workspace path is empty."
  }
  return (Resolve-Path -LiteralPath $Path).Path
}

function Ensure-PlanFile {
  param([string]$Path)
  if (Test-Path -LiteralPath $Path) {
    return $true
  }
  $template = @"
# PLAN
# Write the next batch of tasks for Codex here.
# Example:
# - Finish ToolSettings integration end-to-end.
# - Implement SecuritySettings allowlist and hook into backend.
"@
  Set-Content -LiteralPath $Path -Value $template -Encoding UTF8
  Write-Host "Created $Path. Edit it and rerun the script."
  return $false
}

function Ensure-CodexAvailable {
  $cmd = Get-Command codex -ErrorAction SilentlyContinue
  if (-not $cmd) {
    throw "codex command not found in PATH."
  }
}

function Read-FileSafe {
  param([string]$Path)
  if (Test-Path -LiteralPath $Path) {
    return (Get-Content -LiteralPath $Path -Raw -Encoding UTF8)
  }
  return ""
}

function Write-LoopWarning {
  param([string]$Message)
  if ($script:LogFull) {
    Add-Content -LiteralPath $script:LogFull -Value ("[WARN] " + $Message + "`n") -Encoding UTF8
  }
  Write-Warning $Message
}

function Try-RegexMatch {
  param(
    [string]$Text,
    [string]$Pattern,
    [string]$Context = ""
  )
  try {
    return [regex]::Match($Text, $Pattern)
  } catch {
    $where = if ($Context) { $Context } else { "regex" }
    Write-LoopWarning ("Regex error in " + $where + ": " + $_.Exception.Message + " Pattern=" + $Pattern)
    return $null
  }
}

function Get-NumberedOptions {
  param([string]$Text)
  $options = @()
  if ([string]::IsNullOrWhiteSpace($Text)) {
    return $options
  }
  $pattern = '^\s*(\d{1,2})[\.\)\]:\-\u3001]\s+(.+)$'
  foreach ($line in ($Text -split "`r?`n")) {
    $match = Try-RegexMatch -Text $line -Pattern $pattern -Context "Get-NumberedOptions"
    if ($match -and $match.Success) {
      $options += [pscustomobject]@{
        Number = [int]$match.Groups[1].Value
        Text   = $match.Groups[2].Value.Trim()
      }
    }
  }
  return $options
}

function Has-DecisionCue {
  param([string]$Text)
  if ([string]::IsNullOrWhiteSpace($Text)) {
    return $false
  }
  $lower = $Text.ToLowerInvariant()
  $english = @(
    "choose",
    "select",
    "pick",
    "option",
    "reply with",
    "respond with",
    "which",
    "number"
  )
  foreach ($word in $english) {
    if ($lower -like "*$word*") {
      return $true
    }
  }
  $qm = Try-RegexMatch -Text $Text -Pattern '[\?\uFF1F]' -Context "Has-DecisionCue"
  if ($qm -and $qm.Success) {
    return $true
  }
  $patterns = @(
    '\u9009\u62e9',
    '\u9009\u9879',
    '\u56de\u590d',
    '\u7f16\u53f7',
    '\u8bf7\u8f93\u5165',
    '\u8bf7\u9009',
    '\u8fdb\u5165\u54ea\u4e2a'
  )
  foreach ($pattern in $patterns) {
    $match = Try-RegexMatch -Text $Text -Pattern $pattern -Context "Has-DecisionCue"
    if ($match -and $match.Success) {
      return $true
    }
  }
  $cjk = Try-RegexMatch -Text $Text -Pattern '\p{IsCJKUnifiedIdeographs}' -Context "Has-DecisionCue"
  if ($cjk -and $cjk.Success) {
    return $true
  }
  return $false
}

function Needs-Decision {
  param([string]$Text)
  $options = Get-NumberedOptions -Text $Text
  if ($options.Count -lt 2) {
    return $false
  }
  if (Has-DecisionCue -Text $Text) {
    return $true
  }
  return $false
}

function Build-DecisionPrompt {
  param(
    [string]$LastResponse,
    [object[]]$Options
  )
  $optionLines = $Options | ForEach-Object { "$($_.Number). $($_.Text)" } | Out-String
  return @"
You are a decision-only helper.
Choose the single best option number to proceed.
Rules:
- Reply with only the number (e.g. 1).
- Do not include any other text.
- Do not modify files or run commands.

Assistant response:
<<<
$LastResponse
>>>

Options:
$optionLines
"@
}

function Parse-DecisionNumber {
  param(
    [string]$DecisionText,
    [object[]]$Options
  )
  if ([string]::IsNullOrWhiteSpace($DecisionText)) {
    return $null
  }
  $match = [regex]::Match($DecisionText, '\d+')
  if (-not $match.Success) {
    return $null
  }
  $num = [int]$match.Value
  if ($Options | Where-Object { $_.Number -eq $num }) {
    return $num
  }
  return $null
}

function Invoke-Codex {
  param(
    [string]$Prompt,
    [string]$OutputFile
  )
  if ([string]::IsNullOrWhiteSpace($Prompt)) {
    throw "Prompt is empty."
  }
  $args = @(
    "exec",
    "--cd", $script:WorkspaceFull,
    "--output-last-message", $OutputFile,
    "--color", "never"
  )
  if ($FullAuto) {
    $args += "--full-auto"
  }
  if ($Profile) {
    $args += @("--profile", $Profile)
  }
  $Prompt | codex @args - | ForEach-Object { Write-Host $_ }
  $exitCode = $LASTEXITCODE
  return $exitCode
}

function Build-ContinuationPrompt {
  param(
    [string]$PlanText,
    [string]$LastResponse,
    [int]$DecisionNumber
  )
  return @"
Plan:
$PlanText

Previous assistant response:
<<<
$LastResponse
>>>

User decision: $DecisionNumber

Continue with the selected option. If more choices are required, ask again using a numbered list.
"@
}

function Invoke-DecisionLoop {
  param(
    [string]$LastResponse
  )
  $current = $LastResponse
  $exitCode = 0

  if (-not $AutoDecide) {
    return @{ Last = $current; ExitCode = 0 }
  }

  for ($round = 1; $round -le $DecisionRounds; $round++) {
    if (-not (Needs-Decision -Text $current)) {
      break
    }

    $options = Get-NumberedOptions -Text $current
    if ($options.Count -lt 2) {
      break
    }

    Add-Content -LiteralPath $script:LogFull -Value ("`n### Auto-decision round {0}`n" -f $round) -Encoding UTF8
    $decisionPrompt = Build-DecisionPrompt -LastResponse $current -Options $options
    $decisionExit = Invoke-Codex -Prompt $decisionPrompt -OutputFile $script:DecisionFull
    $decisionText = Read-FileSafe -Path $script:DecisionFull
    Add-Content -LiteralPath $script:LogFull -Value ($decisionText.Trim() + "`n") -Encoding UTF8

    if ($decisionExit -ne 0) {
      $exitCode = $decisionExit
      break
    }

    $decisionNumber = Parse-DecisionNumber -DecisionText $decisionText -Options $options
    if (-not $decisionNumber) {
      $decisionNumber = $options[0].Number
      Add-Content -LiteralPath $script:LogFull -Value ("Decision parse failed. Defaulting to {0}.`n" -f $decisionNumber) -Encoding UTF8
    }

    $planText = Read-FileSafe -Path $script:PlanFull
    $continuePrompt = Build-ContinuationPrompt -PlanText $planText -LastResponse $current -DecisionNumber $decisionNumber
    $exitCode = Invoke-Codex -Prompt $continuePrompt -OutputFile $script:LastFull
    $current = Read-FileSafe -Path $script:LastFull
    Add-Content -LiteralPath $script:LogFull -Value ($current.Trim() + "`n") -Encoding UTF8

    if ($exitCode -ne 0) {
      break
    }
  }

  return @{ Last = $current; ExitCode = $exitCode }
}

function Has-ErrorCue {
  param([string]$Text)
  if ([string]::IsNullOrWhiteSpace($Text)) {
    return $false
  }
  $patterns = @(
    'error',
    'failed',
    'failure',
    'exception',
    'traceback',
    'panic',
    'stack trace',
    'tests? failed',
    'build failed',
    'compilation failed',
    'lint failed',
    'exit code',
    'non-zero',
    '\u5931\u8d25',
    '\u9519\u8bef',
    '\u5f02\u5e38',
    '\u65e0\u6cd5',
    '\u627e\u4e0d\u5230',
    '\u9519\u8bef\u7801'
  )
  foreach ($pattern in $patterns) {
    $match = Try-RegexMatch -Text $Text -Pattern $pattern -Context "Has-ErrorCue"
    if ($match -and $match.Success) {
      return $true
    }
  }
  return $false
}

function Build-RepairPrompt {
  param(
    [string]$PlanText,
    [string]$LastResponse,
    [string]$Reason
  )
  return @"
You are a repair-only helper.
The last run hit an error and must be fixed automatically.

Reason:
$Reason

Plan:
$PlanText

Last assistant response:
<<<
$LastResponse
>>>

Instructions:
- Diagnose the error.
- Apply minimal fixes in the repo.
- Run any necessary checks if they are cheap.
- Do not ask questions unless blocked.
- Summarize changes briefly at the end.
"@
}

function Invoke-Iteration {
  param(
    [int]$Index,
    [bool]$IsLast
  )
  $stamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
  Add-Content -LiteralPath $script:LogFull -Value ("`n## Run {0} - {1}`n" -f $Index, $stamp) -Encoding UTF8

  $prompt = Read-FileSafe -Path $script:PlanFull
  if (-not $prompt.Trim()) {
    Add-Content -LiteralPath $script:LogFull -Value "Plan is empty. Aborting.`n" -Encoding UTF8
    return $false
  }

  $exitCode = Invoke-Codex -Prompt $prompt -OutputFile $script:LastFull
  $last = Read-FileSafe -Path $script:LastFull
  if ($last) {
    Add-Content -LiteralPath $script:LogFull -Value ($last.Trim() + "`n") -Encoding UTF8
  }

  $decisionResult = Invoke-DecisionLoop -LastResponse $last
  $last = $decisionResult.Last
  if ($decisionResult.ExitCode -ne 0) {
    $exitCode = $decisionResult.ExitCode
  }

  $needsRepair = $AutoRepair -and (($exitCode -ne 0) -or (Has-ErrorCue -Text $last))
  if ($needsRepair) {
    for ($r = 1; $r -le $RepairRounds; $r++) {
      Add-Content -LiteralPath $script:LogFull -Value ("`n### Auto-repair round {0}`n" -f $r) -Encoding UTF8
      $reason = if ($exitCode -ne 0) { "Codex exited with code $exitCode" } else { "Error indicators found in assistant output." }
      $repairPrompt = Build-RepairPrompt -PlanText $prompt -LastResponse $last -Reason $reason
      $repairExit = Invoke-Codex -Prompt $repairPrompt -OutputFile $script:RepairFull
      $repairText = Read-FileSafe -Path $script:RepairFull
      if ($repairText) {
        Add-Content -LiteralPath $script:LogFull -Value ($repairText.Trim() + "`n") -Encoding UTF8
      }

      if ($repairExit -ne 0) {
        Add-Content -LiteralPath $script:LogFull -Value ("Repair Codex exited with code {0}.`n" -f $repairExit) -Encoding UTF8
        $exitCode = $repairExit
        break
      }

      if ($RepairDelaySeconds -gt 0) {
        Start-Sleep -Seconds $RepairDelaySeconds
      }

      $exitCode = Invoke-Codex -Prompt $prompt -OutputFile $script:LastFull
      $last = Read-FileSafe -Path $script:LastFull
      if ($last) {
        Add-Content -LiteralPath $script:LogFull -Value ($last.Trim() + "`n") -Encoding UTF8
      }

      $decisionResult = Invoke-DecisionLoop -LastResponse $last
      $last = $decisionResult.Last
      if ($decisionResult.ExitCode -ne 0) {
        $exitCode = $decisionResult.ExitCode
      }

      if ($exitCode -eq 0 -and -not (Has-ErrorCue -Text $last)) {
        break
      }
    }
  }

  if ($exitCode -ne 0) {
    Add-Content -LiteralPath $script:LogFull -Value ("Codex exited with code {0}.`n" -f $exitCode) -Encoding UTF8
    if ($ContinueOnError) {
      return $true
    }
    return $false
  }

  if ($DelaySeconds -gt 0 -and -not $IsLast) {
    Start-Sleep -Seconds $DelaySeconds
  }

  return $true
}

try {
  Ensure-CodexAvailable
  $script:WorkspaceFull = Resolve-WorkspacePath -Path $Workspace
  Push-Location $script:WorkspaceFull

  $script:PlanFull = Join-Path $script:WorkspaceFull $PlanPath
  $script:LogFull = Join-Path $script:WorkspaceFull $LogPath
  $script:LastFull = Join-Path $script:WorkspaceFull $LastMessagePath
  $script:DecisionFull = Join-Path $script:WorkspaceFull $DecisionMessagePath
  $script:RepairFull = Join-Path $script:WorkspaceFull $RepairMessagePath

  if (-not (Ensure-PlanFile -Path $script:PlanFull)) {
    exit 1
  }

  if ($Forever) {
    $i = 1
    while ($true) {
      if (-not (Invoke-Iteration -Index $i -IsLast $false)) {
        break
      }
      $i += 1
    }
  } else {
    for ($i = 1; $i -le $Iterations; $i++) {
      $isLast = $i -ge $Iterations
      if (-not (Invoke-Iteration -Index $i -IsLast $isLast)) {
        break
      }
    }
  }
} finally {
  Pop-Location
}
