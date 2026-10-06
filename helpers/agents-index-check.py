#!/usr/bin/env python3
# Fail when the five lists that name the agent instructions disagree.
#
#     helpers/agents-index-check.py        # script/lint runs it; exit 1 on any disagreement
#
# Why this exists. On 2026-10-06 (UTC) the rules that only one kind of work needs moved out of
# AGENTS.md into notes/skills/<name>/SKILL.md (calef's ruling: agent-agnostic, because "we expect
# multiple agents and multiple contributors and who knows what their toolset will be"). From then on
# the same set of names is written in five places, and each one is how a different reader finds them:
#
#   1. the directories notes/skills/<name>/, each with a SKILL.md whose frontmatter `name` matches;
#   2. the table in notes/skills/README.md, whose description column must equal each frontmatter
#      description word for word (calef asked for the same text, so a reader of either sees one);
#   3. AGENTS.md's "read when" links, which an agent that only reads files depends on;
#   4. the symlinks .claude/skills/<name> -> ../../notes/skills/<name> (Claude Code);
#   5. the symlinks .agents/skills/<name> -> ../../notes/skills/<name> (the cross-client
#      convention agentskills.io describes, read by Codex and opencode among others).
#
# A new instruction added to one place and not the others is invisible to whichever reader uses a
# list it is missing from, and nobody would notice. This is that drift, made a gate (the ladder's
# rung two). It also holds the Agent Skills specification's two hard limits that a tool may reject
# a skill over: the name matches its directory, and the description is 1 to 1,024 characters.
#
# Name: provisional, minted 2026-10-06 by lane/agents-md-skills; calef has not ruled on it.
#
# BUGS. It reads the frontmatter description by hand (no YAML library is guaranteed here), so it
# understands a plain `description:` line or a `>-` folded block and nothing fancier. It does not
# check that AGENTS.md's "read when" wording matches the README's "When to read it" column, only
# that both name the same set.

import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DIR = 'notes/skills'
LINK_DIRS = ('.claude/skills', '.agents/skills')


def frontmatter(path):
    text = open(path, encoding='utf-8').read()
    m = re.match(r'---\n(.*?)\n---\n', text, re.S)
    if not m:
        return None, None
    name = desc = None
    lines = m.group(1).split('\n')
    for i, line in enumerate(lines):
        if line.startswith('name:'):
            name = line[5:].strip()
        elif line.startswith('description:'):
            rest = line[12:].strip()
            if rest in ('>-', '>', '|', '|-'):
                block = []
                for nxt in lines[i + 1:]:
                    if not nxt.startswith(' '):
                        break
                    block.append(nxt.strip())
                desc = ' '.join(block)
            else:
                desc = rest
    return name, desc


def main():
    bad = []
    base = os.path.join(ROOT, DIR)
    dirs = sorted(d for d in os.listdir(base) if os.path.isdir(os.path.join(base, d)))
    if not dirs:
        sys.exit(f'agents index: {DIR}/ holds no directories, so nothing was judged')

    descs = {}
    for d in dirs:
        p = os.path.join(base, d, 'SKILL.md')
        if not os.path.isfile(p):
            bad.append(f'{DIR}/{d}/ has no SKILL.md')
            continue
        name, desc = frontmatter(p)
        if name != d:
            bad.append(f'{DIR}/{d}/SKILL.md: frontmatter name is {name!r}, not the directory name')
        if not desc or len(desc) > 1024:
            bad.append(f'{DIR}/{d}/SKILL.md: description is {len(desc or "")} characters; '
                       'the Agent Skills specification allows 1 to 1,024')
        descs[d] = desc
    names = set(dirs)

    rows = {}
    readme = open(os.path.join(base, 'README.md'), encoding='utf-8').read()
    for m in re.finditer(r'^\| \[`([a-z0-9-]+)`\]\(\1/SKILL\.md\) \| (.*?) \| (.*?) \|$', readme, re.M):
        rows[m.group(1)] = m.group(2)
    for n in sorted(names ^ set(rows)):
        bad.append(f'{DIR}/README.md: row for {n!r} ' + ('missing' if n in names else 'has no directory'))
    for n in sorted(names & set(rows)):
        if rows[n] != descs.get(n):
            bad.append(f'{DIR}/README.md: the description for {n!r} differs from its frontmatter')

    agents = open(os.path.join(ROOT, 'AGENTS.md'), encoding='utf-8').read()
    linked = set(re.findall(r'\]\(notes/skills/([a-z0-9-]+)/SKILL\.md\)', agents))
    for n in sorted(names ^ linked):
        bad.append(f'AGENTS.md: link to {DIR}/{n}/SKILL.md ' + ('missing' if n in names else 'names no directory'))

    for ld in LINK_DIRS:
        p = os.path.join(ROOT, ld)
        entries = set(os.listdir(p)) if os.path.isdir(p) else set()
        for n in sorted(names ^ entries):
            bad.append(f'{ld}/{n} ' + ('missing' if n in names else f'has no {DIR}/{n}'))
        for n in sorted(names & entries):
            q = os.path.join(p, n)
            want = f'../../{DIR}/{n}'
            if not os.path.islink(q) or os.readlink(q) != want:
                bad.append(f'{ld}/{n} is not a symlink to {want}')

    if bad:
        print('lint: the agent instruction lists disagree:', file=sys.stderr)
        for b in bad:
            print(f'  {b}', file=sys.stderr)
        print(f'Add or remove an instruction in all five places: {DIR}/<name>/SKILL.md, the table in '
              f'{DIR}/README.md, its link in AGENTS.md, and the symlinks in {" and ".join(LINK_DIRS)}.',
              file=sys.stderr)
        sys.exit(1)
    print(f'agents index: {len(names)} instructions, each in its directory, the README, AGENTS.md '
          f'and both symlink directories')


if __name__ == '__main__':
    main()
