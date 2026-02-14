#!/usr/bin/env -S deno run --allow-run --allow-read --allow-write

/**
 * Generates markdown documentation from `xin --help` output.
 * Run periodically as the CLI evolves to keep skill references up to date.
 * 
 * Usage:
 *   deno run --allow-run --allow-read --allow-write scripts/generate-docs.ts
 */

import * as path from "jsr:@std/path"

const SCRIPT_DIR = path.dirname(new URL(import.meta.url).pathname)
const SKILL_DIR = path.join(SCRIPT_DIR, "..")
const REFERENCES_DIR = path.join(SKILL_DIR, "references")
const SKILL_MD = path.join(SKILL_DIR, "SKILL.md")
// SKILL template is kept in scripts/templates (not part of the skill itself)
const SKILL_TEMPLATE = path.join(SCRIPT_DIR, "templates", "SKILL.template.md")

// Files to preserve (not generated from CLI help)
// Keep these stable, hand-edited references around when regenerating.
const PRESERVED_FILES: string[] = [
  "common-tasks.md",
  // commands.md is manually curated (merged with high-level overview)
  "commands.md",
  // _schemas/ - JSON schemas for agents (not generated, hand-crafted)
  "_schemas",
]

// Commands to skip (not useful for docs, or pseudo-commands like clap's `help`)
const SKIP_COMMANDS_ANYWHERE: Set<string> = new Set(["help"])

interface CommandInfo {
  name: string
  description: string
  help: string
  subcommands: CommandInfo[]
}

async function run(
  cmd: string[],
): Promise<{ success: boolean; stdout: string; stderr: string }> {
  const command = new Deno.Command(cmd[0], {
    args: cmd.slice(1),
    stdout: "piped",
    stderr: "piped",
    env: { "NO_COLOR": "1" }, // Disable ANSI colors
  })
  const result = await command.output()
  return {
    success: result.success,
    stdout: new TextDecoder().decode(result.stdout).trim(),
    stderr: new TextDecoder().decode(result.stderr).trim(),
  }
}

function stripAnsi(str: string): string {
  // Remove ANSI escape codes (in case NO_COLOR doesn't work)
  // deno-lint-ignore no-control-regex
  return str.replace(/\x1b\[[0-9;]*m/g, "")
}

function parseCommands(helpText: string): string[] {
  const commands: string[] = []
  const lines = helpText.split("\n")
  let inCommands = false
  let foundAnyCommand = false

  for (const line of lines) {
    if (line.startsWith("Commands:")) {
      inCommands = true
      continue
    }
    if (inCommands) {
      // Command lines look like: "  command, alias  - Description"
      // or: "  command  <arg>  - Description"
      // Capture command name (stopping at comma or whitespace)
      const match = line.match(/^\s{2}([a-z][-a-z0-9]*)(?:,|\s)/)
      if (match) {
        commands.push(match[1])
        foundAnyCommand = true
      } else if (foundAnyCommand && line.trim() === "") {
        // Empty line after we've found commands means end of section
        break
      }
      // Skip empty lines before first command (common in help output)
    }
  }

  return commands
}

async function getCommandHelp(cmdPath: string[], xinBin: string): Promise<string> {
  const result = await run([xinBin, ...cmdPath, "--help"])
  if (!result.success) {
    return result.stderr || "Command help not available"
  }
  return stripAnsi(result.stdout)
}

function extractDescriptionFromHelp(help: string): string {
  // Some CLIs (like linear) have a "Description:" section, but clap-based help
  // (like xin) typically starts with a one-line summary.
  const descMatch = help.match(/Description:\s*\n\s*(.+)/)
  if (descMatch) return descMatch[1].trim()

  const firstNonEmpty = help.split("\n").find((l) => l.trim().length > 0)
  return (firstNonEmpty ?? "").trim()
}

async function discoverCommand(
  cmdPath: string[],
  xinBin: string,
): Promise<CommandInfo> {
  const help = await getCommandHelp(cmdPath, xinBin)
  const name = cmdPath.join(" ")

  const description = extractDescriptionFromHelp(help)

  // Find subcommands
  const subcommandNames = parseCommands(help).filter(
    (c) => !SKIP_COMMANDS_ANYWHERE.has(c),
  )
  const subcommands: CommandInfo[] = []

  for (const subcmd of subcommandNames) {
    const subInfo = await discoverCommand([...cmdPath, subcmd], xinBin)
    subcommands.push(subInfo)
  }

  return { name, description, help, subcommands }
}

function generateCommandDoc(cmd: CommandInfo): string {
  const lines: string[] = []
  const cmdName = cmd.name.replace(/^xin /, "")

  lines.push(`# ${cmdName}`)
  lines.push("")
  lines.push(`> ${cmd.description}`)
  lines.push("")
  lines.push("## Usage")
  lines.push("")
  lines.push("```")
  lines.push(cmd.help)
  lines.push("```")

  // Add subcommand details
  if (cmd.subcommands.length > 0) {
    lines.push("")
    lines.push("## Subcommands")

    for (const sub of cmd.subcommands) {
      const subName = sub.name.split(" ").pop()!
      lines.push("")
      lines.push(`### ${subName}`)
      lines.push("")
      if (sub.description) {
        lines.push(`> ${sub.description}`)
        lines.push("")
      }
      lines.push("```")
      lines.push(sub.help)
      lines.push("```")

      // Handle 3-level deep commands (e.g., inbox do)
      if (sub.subcommands.length > 0) {
        lines.push("")
        lines.push(`#### ${subName} subcommands`)

        for (const subsub of sub.subcommands) {
          const subsubName = subsub.name.split(" ").pop()!
          lines.push("")
          lines.push(`##### ${subsubName}`)
          lines.push("")
          lines.push("```")
          lines.push(subsub.help)
          lines.push("```")
        }
      }
    }
  }

  return lines.join("\n")
}

async function getXinVersion(xinBin: string): Promise<string> {
  const result = await run([xinBin, "--version"])
  if (!result.success) return "unknown"
  // Parse version from output (e.g., "xin 0.1.0")
  const match = stripAnsi(result.stdout).match(/xin\s+(\S+)/)
  return match ? match[1] : "unknown"
}

async function main() {
  console.log("Generating xin CLI documentation...")

  // Check xin is available - try both local build and system PATH
  let xinBin = "xin"
  const localBuild = path.join(SCRIPT_DIR, "..", "..", "..", "target", "debug", "xin")
  try {
    const testResult = await run([localBuild, "--version"])
    if (testResult.success) {
      xinBin = localBuild
      console.log(`Using local build: ${xinBin}`)
    }
  } catch {
    // Fall back to system xin
  }

  const versionResult = await run([xinBin, "--version"])
  if (!versionResult.success) {
    console.error("Error: xin CLI not found. Is it installed or built?")
    Deno.exit(1)
  }

  const version = await getXinVersion(xinBin)
  console.log(`xin CLI version: ${version}`)

  // Auto-discover top-level commands from `xin --help`
  console.log("Discovering commands...")
  const topLevelHelp = await getCommandHelp([], xinBin)
  const topLevelCommands = parseCommands(topLevelHelp).filter(
    (cmd) => !SKIP_COMMANDS_ANYWHERE.has(cmd),
  )
  console.log(`Found ${topLevelCommands.length} top-level commands`)

  const commands: CommandInfo[] = []

  for (const cmd of topLevelCommands) {
    console.log(`  Discovering: ${cmd}`)
    const info = await discoverCommand([cmd], xinBin)
    commands.push(info)
  }

  // Generate markdown files
  console.log("Generating markdown files...")

  // Ensure references directory exists
  await Deno.mkdir(REFERENCES_DIR, { recursive: true })

  // Get list of preserved paths to keep (files and directories)
  const preservedPaths = new Set(
    PRESERVED_FILES.map((f) => path.join(REFERENCES_DIR, f)),
  )

  // Clean up old generated files (but preserve manual files and directories)
  for await (const entry of Deno.readDir(REFERENCES_DIR)) {
    const filePath = path.join(REFERENCES_DIR, entry.name)
    // Skip if it's a preserved file OR a preserved directory
    const isPreserved = preservedPaths.has(filePath)
    if (!isPreserved && entry.name.endsWith(".md")) {
      await Deno.remove(filePath)
    }
  }

  // Write command documentation
  for (const cmd of commands) {
    const filename = `${cmd.name.replace(/^xin /, "")}.md`
    const filepath = path.join(REFERENCES_DIR, filename)
    const content = generateCommandDoc(cmd)
    await Deno.writeTextFile(filepath, content + "\n")
    console.log(`  Generated: ${filename}`)
  }

  // Generate index file
  const indexContent = generateIndex(commands, version)
  await Deno.writeTextFile(path.join(REFERENCES_DIR, "commands.md"), indexContent)
  console.log("  Generated: commands.md")

  // Generate SKILL.md from template
  try {
    await Deno.stat(SKILL_TEMPLATE)
    console.log("Generating SKILL.md from template...")
    const skillContent = await generateSkillMd(commands, version)
    await Deno.writeTextFile(SKILL_MD, skillContent)
    console.log("  Generated: SKILL.md")
  } catch {
    console.log("SKILL.template.md not found, skipping SKILL.md generation")
  }

  console.log(`\nDone! Generated ${commands.length + 1} files.`)
}

function generateIndex(commands: CommandInfo[], version: string): string {
  const lines: string[] = []

  lines.push("# xin CLI Command Reference")
  lines.push("")
  lines.push(`> Generated from xin CLI v${version}`)
  lines.push("")
  lines.push("xin is an agent-first JMAP CLI for Fastmail. It provides JSON-first output")
  lines.push("as the stable contract, with `--plain` for human-friendly output.")
  lines.push("")
  lines.push("## Commands")
  lines.push("")

  for (const cmd of commands) {
    const cmdName = cmd.name.replace(/^xin /, "")
    lines.push(`- [${cmdName}](./${cmdName}.md) - ${cmd.description}`)
  }

  lines.push("")
  lines.push("## Quick Reference")
  lines.push("")
  lines.push("```bash")
  lines.push("# Get help for any command")
  lines.push("xin <command> --help")
  lines.push("xin <command> <subcommand> --help")
  lines.push("")
  lines.push("# JSON is the stable contract (default)")
  lines.push("xin search \"from:alice seen:false\" --max 10")
  lines.push("")
  lines.push("# --plain is for humans (not a stability contract)")
  lines.push("xin --plain search \"subject:invoice\" --max 5")
  lines.push("```")

  return lines.join("\n") + "\n"
}

function generateCommandsSection(commands: CommandInfo[]): string {
  const lines: string[] = []
  lines.push("```")

  // Find max command name length for alignment
  const maxLen = Math.max(...commands.map((c) => c.name.length))

  for (const cmd of commands) {
    const padding = " ".repeat(maxLen - cmd.name.length + 2)
    lines.push(`xin ${cmd.name}${padding}# ${cmd.description}`)
  }

  lines.push("```")
  return lines.join("\n")
}

function generateReferenceToc(commands: CommandInfo[]): string {
  const lines: string[] = []

  for (const cmd of commands) {
    lines.push(
      `- [${cmd.name}](references/${cmd.name}.md) - ${cmd.description}`,
    )
  }

  return lines.join("\n")
}

async function generateSkillMd(
  commands: CommandInfo[],
  version: string,
): Promise<string> {
  const template = await Deno.readTextFile(SKILL_TEMPLATE)
  return template
    .replace("{{COMMANDS}}", generateCommandsSection(commands))
    .replace("{{REFERENCE_TOC}}", generateReferenceToc(commands))
    .replace("{{VERSION}}", version)
}

main()
