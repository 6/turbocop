#!/usr/bin/env python3
"""Exclude project instructions and skills from CI agent context.

Called by the run-agent composite action before the agent starts.
Configures both Claude and Codex backends so that project skills
and instruction files are not loaded into the agent's context.
"""

from __future__ import annotations

import json
from pathlib import Path


def configure_claude(repo_root: Path) -> dict:
    """Exclude CLAUDE.md/AGENTS.md and deny the Skill tool.

    Returns the final settings dict written to disk.
    """
    settings_path = repo_root / ".claude" / "settings.local.json"
    cfg: dict = json.loads(settings_path.read_text()) if settings_path.exists() else {}
    cfg["claudeMdExcludes"] = ["**/CLAUDE.md", "**/AGENTS.md"]
    deny = cfg.setdefault("permissions", {}).setdefault("deny", [])
    if "Skill" not in deny:
        deny.append("Skill")
    settings_path.write_text(json.dumps(cfg, indent=2) + "\n")
    return cfg


def configure_codex(repo_root: Path, codex_home: Path | None = None) -> int:
    """Disable project skills via ~/.codex/config.toml.

    Returns the number of skills disabled.
    """
    skills_dir = repo_root / ".agents" / "skills"
    if not skills_dir.is_dir():
        return 0
    skills = sorted(skills_dir.glob("*/SKILL.md"))
    if not skills:
        return 0

    if codex_home is None:
        codex_home = Path.home() / ".codex"
    codex_home.mkdir(parents=True, exist_ok=True)
    cfg_path = codex_home / "config.toml"

    parts: list[str] = []
    if cfg_path.exists():
        existing = cfg_path.read_text()
        if existing.strip():
            parts.append(existing.rstrip("\n"))

    for skill in skills:
        abs_path = skill.resolve()
        parts.append(
            f'[[skills.config]]\npath = "{abs_path}"\nenabled = false'
        )

    cfg_path.write_text("\n\n".join(parts) + "\n")
    return len(skills)


def main() -> None:
    import argparse

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--repo-root",
        type=Path,
        default=Path.cwd(),
        help="Repository root (default: cwd)",
    )
    args = parser.parse_args()

    configure_claude(args.repo_root)
    count = configure_codex(args.repo_root)
    if count:
        print(f"Disabled {count} Codex skill(s)")


if __name__ == "__main__":
    main()
