#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

version="$(tr -d '[:space:]' < VERSION)"
tag="v$version"
if git rev-parse "$tag" >/dev/null 2>&1; then
  echo "Tag $tag already exists."
  exit 1
fi

if [[ -n "$(git status --porcelain)" ]]; then
  echo "Working tree is dirty. Commit first."
  exit 1
fi

git tag "$tag"
git push origin "$tag"
echo "Pushed $tag. GitHub Actions will build the release."
