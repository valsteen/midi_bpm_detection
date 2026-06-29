#!/usr/bin/env zsh
set -u

audit_path=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --audit-path)
      if [[ $# -lt 2 ]]; then
        print -u2 "missing value for --audit-path"
        exit 2
      fi
      audit_path="$2"
      shift 2
      ;;
    --help|-h)
      cat <<'HELP'
Usage: zsh coordinator-status.zsh [--audit-path .codex/audits/<audit-name>]

Prints one deterministic, local-only coordinator status packet:
- current branch, upstream, ahead/behind counts, and HEAD
- working tree changed-file summary
- whether .codex/audits is ignored
- tracked transient audit artifacts under docs/
- hot-path and queue line counts when --audit-path is provided

This helper never fetches, pushes, pulls, or queries GitHub.
HELP
      exit 0
      ;;
    *)
      print -u2 "unknown argument: $1"
      exit 2
      ;;
  esac
done

repo_root="$(git rev-parse --show-toplevel 2>/dev/null)" || {
  print -u2 "not inside a git repository"
  exit 1
}

cd "$repo_root" || exit 1

print "coordinator_status_version=1"
print "repo_root=$repo_root"

branch="$(git branch --show-current 2>/dev/null)"
head_short="$(git rev-parse --short HEAD 2>/dev/null)"
upstream="$(git rev-parse --abbrev-ref --symbolic-full-name '@{u}' 2>/dev/null || true)"

print ""
print "[branch]"
print "branch=${branch:-DETACHED}"
print "head=${head_short:-unknown}"
if [[ -n "$upstream" ]]; then
  counts="$(git rev-list --left-right --count "${upstream}...HEAD" 2>/dev/null || true)"
  if [[ -n "$counts" ]]; then
    behind="${counts%%[[:space:]]*}"
    ahead="${counts##*[[:space:]]}"
    print "upstream=$upstream"
    print "ahead=$ahead"
    print "behind=$behind"
  else
    print "upstream=$upstream"
    print "ahead=unknown"
    print "behind=unknown"
  fi
else
  print "upstream=none"
fi

print ""
print "[working_tree]"
changed_count="$(git status --porcelain=v1 2>/dev/null | wc -l | tr -d ' ')"
print "changed_files=$changed_count"
if [[ "$changed_count" == "0" ]]; then
  print "changes=none"
else
  git status --short
fi

print ""
print "[audit_state]"
if git check-ignore -q .codex/audits 2>/dev/null; then
  print "codex_audits_ignored=yes"
else
  print "codex_audits_ignored=no"
fi

tracked_transient="$(git ls-files 'docs/audits/**' 'docs/*handoff*.md' 2>/dev/null || true)"
if [[ -n "$tracked_transient" ]]; then
  transient_count="$(print -r -- "$tracked_transient" | wc -l | tr -d ' ')"
  print "tracked_transient_docs=$transient_count"
  print -r -- "$tracked_transient"
else
  print "tracked_transient_docs=0"
fi

if [[ -n "$audit_path" ]]; then
  print ""
  print "[hot_path]"
  for file_name in current.md active-slice.md; do
    file_path="$audit_path/$file_name"
    if [[ -f "$file_path" ]]; then
      line_count="$(wc -l < "$file_path" | tr -d ' ')"
      case "$file_name" in
        current.md)
          hard_stop=150
          ;;
        active-slice.md)
          hard_stop=300
          ;;
      esac
      if (( line_count > hard_stop )); then
        state="over_hard_stop"
      else
        state="ok"
      fi
      print "$file_name=$line_count/$hard_stop:$state"
      if [[ "$(sed -n '1p' "$file_path")" == "---" ]]; then
        awk -v prefix="$file_name" '
          NR == 1 { next }
          /^---$/ { exit }
          /^(kind|state|item|active_ref|next_action|read_policy|updated):[[:space:]]*/ {
            key = $1
            sub(":", "", key)
            value = $0
            sub("^[^:]+:[[:space:]]*", "", value)
            print prefix "." key "=" value
          }
        ' "$file_path"
      else
        print "$file_name.protocol=legacy_format"
      fi
    else
      print "$file_name=missing"
    fi
  done

  queue_path="$audit_path/queue.md"
  print ""
  print "[queue]"
  if [[ -f "$queue_path" ]]; then
    queue_lines="$(wc -l < "$queue_path" | tr -d ' ')"
    print "queue.md=$queue_lines"
    if [[ "$(sed -n '1p' "$queue_path")" == "---" ]]; then
      awk '
        NR == 1 { next }
        /^---$/ { exit }
        /^(kind|state|item|updated|next_action|read_policy):[[:space:]]*/ {
          key = $1
          sub(":", "", key)
          value = $0
          sub("^[^:]+:[[:space:]]*", "", value)
          print "queue.md." key "=" value
        }
      ' "$queue_path"
    else
      print "queue.md.protocol=legacy_format"
    fi
    active_count="$(awk -F'|' '
      /^\|/ && $0 !~ /^\|[ -]+\|/ && tolower($0) !~ /\|[[:space:]]*state[[:space:]]*\|/ {
        state = tolower($3)
        gsub(/^[[:space:]]+|[[:space:]]+$/, "", state)
        if (state != "" && state != "done" && state != "superseded" && state != "dropped") count++
      }
      END { print count + 0 }
    ' "$queue_path")"
    print "queue.active_items=$active_count"
  else
    print "queue.md=missing"
  fi
fi
