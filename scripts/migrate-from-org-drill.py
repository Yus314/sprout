#!/usr/bin/env python3
"""Migrate Obsidian vault frontmatter from org-drill format to sprout format.

Transformations:
  - date_created → created (rename + unquote)
  - drill_ease → ease (rename, 2 decimal places)
  - drill_last_interval → review_interval (rename, float→int, min 1)
  - next_review, last_review: unquote dates
  - Remove: org_roam_id, drill_repeats_since_fail, drill_total_repeats,
            drill_failure_count, drill_average_quality, bibliography
  - Remove maturity values (seedling/budding/evergreen) from tags list
  - Remove tags field entirely if it becomes empty

Usage:
  python migrate-from-org-drill.py /path/to/vault          # dry run
  python migrate-from-org-drill.py /path/to/vault --apply   # apply changes
"""

import argparse
import math
import re
import sys
from pathlib import Path

FIELDS_TO_REMOVE = {
    "org_roam_id",
    "drill_repeats_since_fail",
    "drill_total_repeats",
    "drill_failure_count",
    "drill_average_quality",
    "bibliography",
}

MATURITY_TAG_VALUES = {"seedling", "budding", "evergreen"}


def parse_frontmatter(content: str) -> tuple[str | None, str]:
    """Split content into (frontmatter_yaml, body). Returns (None, content) if no frontmatter."""
    if not content.startswith("---\n"):
        return None, content
    end = content.find("\n---\n", 4)
    if end == -1:
        return None, content
    yaml = content[4:end + 1]  # between the --- delimiters (includes trailing \n)
    body = content[end + 5:]   # after closing ---\n
    return yaml, body


def migrate_frontmatter(yaml: str) -> str:
    """Transform org-drill YAML frontmatter to sprout format."""
    lines = yaml.splitlines()
    new_lines = []
    i = 0
    in_tags = False
    tag_values = []

    while i < len(lines):
        line = lines[i]

        # Handle multi-line tags block
        if re.match(r"^tags:\s*$", line):
            in_tags = True
            tag_values = []
            i += 1
            while i < len(lines) and re.match(r"^- ", lines[i]):
                tag = lines[i][2:].strip()
                if tag not in MATURITY_TAG_VALUES:
                    tag_values.append(tag)
                i += 1
            if tag_values:
                new_lines.append("tags:")
                for t in tag_values:
                    new_lines.append(f"- {t}")
            continue

        # Handle inline tags (tags: [seedling])
        m = re.match(r"^tags:\s*\[(.+)\]\s*$", line)
        if m:
            tags = [t.strip() for t in m.group(1).split(",")]
            tags = [t for t in tags if t not in MATURITY_TAG_VALUES]
            if tags:
                new_lines.append(f"tags: [{', '.join(tags)}]")
            i += 1
            continue

        # Fields to remove entirely
        key_match = re.match(r"^(\w[\w_]*):", line)
        if key_match:
            key = key_match.group(1)
            if key in FIELDS_TO_REMOVE:
                i += 1
                continue

        # date_created → created (unquote)
        m = re.match(r"^date_created:\s*'?(\d{4}-\d{2}-\d{2})'?\s*$", line)
        if m:
            new_lines.append(f"created: {m.group(1)}")
            i += 1
            continue

        # drill_ease → ease (2 decimal places)
        m = re.match(r"^drill_ease:\s*(\S+)\s*$", line)
        if m:
            ease = float(m.group(1))
            new_lines.append(f"ease: {ease:.2f}")
            i += 1
            continue

        # drill_last_interval → review_interval (float→int, min 1)
        m = re.match(r"^drill_last_interval:\s*(\S+)\s*$", line)
        if m:
            interval = float(m.group(1))
            interval_int = max(1, round(interval))
            new_lines.append(f"review_interval: {interval_int}")
            i += 1
            continue

        # Unquote next_review / last_review dates
        m = re.match(r"^(next_review|last_review):\s*'(\d{4}-\d{2}-\d{2})'\s*$", line)
        if m:
            new_lines.append(f"{m.group(1)}: {m.group(2)}")
            i += 1
            continue

        # Pass through everything else
        new_lines.append(line)
        i += 1

    return "\n".join(new_lines) + "\n"


def migrate_file(path: Path, apply: bool) -> tuple[bool, list[str]]:
    """Migrate a single file. Returns (changed, messages)."""
    content = path.read_text(encoding="utf-8")
    yaml, body = parse_frontmatter(content)

    if yaml is None:
        return False, [f"  SKIP (no frontmatter): {path.name}"]

    new_yaml = migrate_frontmatter(yaml)

    if new_yaml == yaml:
        return False, [f"  SKIP (no changes): {path.name}"]

    new_content = f"---\n{new_yaml}---\n{body}"

    if apply:
        path.write_text(new_content, encoding="utf-8")

    return True, [f"  MIGRATE: {path.name}"]


def main():
    parser = argparse.ArgumentParser(description="Migrate org-drill frontmatter to sprout format")
    parser.add_argument("vault", type=Path, help="Path to the Obsidian vault directory")
    parser.add_argument("--apply", action="store_true", help="Actually write changes (default: dry run)")
    args = parser.parse_args()

    vault = args.vault.resolve()
    if not vault.is_dir():
        print(f"Error: {vault} is not a directory", file=sys.stderr)
        sys.exit(1)

    md_files = sorted(vault.glob("*.md"))
    if not md_files:
        print(f"No .md files found in {vault}", file=sys.stderr)
        sys.exit(1)

    mode = "APPLYING" if args.apply else "DRY RUN"
    print(f"[{mode}] Scanning {len(md_files)} markdown files in {vault}\n")

    changed = 0
    skipped = 0
    for f in md_files:
        was_changed, msgs = migrate_file(f, args.apply)
        for m in msgs:
            print(m)
        if was_changed:
            changed += 1
        else:
            skipped += 1

    print(f"\nSummary: {changed} migrated, {skipped} skipped")
    if not args.apply and changed > 0:
        print("Run with --apply to write changes.")


if __name__ == "__main__":
    main()
