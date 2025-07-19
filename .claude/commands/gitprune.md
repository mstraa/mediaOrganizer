# Git Branch Cleanup

This command will clean up all git branches except 'develop' and 'main' from both local and remote repositories.

## Steps:

1. First, ensure you're on a safe branch (develop or main)
2. Delete all local branches matching patterns: feat/, fix/, feature/, release/, chore/, task/
3. Delete the same branches from remote origin
4. Prune deleted remote branches from local references

## Commands to execute:

```bash
# Switch to develop or main branch first
git checkout develop || git checkout main

# Delete all local branches matching the patterns
git branch | grep -E '^  (feat|fix|feature|release|chore|task|hotfix|docs)/' | xargs -r git branch -D

# Delete any standalone 'release' branch
git branch -D release 2>/dev/null || true

# Get list of remote branches to delete
REMOTE_BRANCHES=$(git branch -r | grep -E 'origin/(feat|fix|feature|release|chore|task|hotfix|docs)/' | sed 's/origin\///' | tr '\n' ' ')

# Delete remote branches if any exist
if [ ! -z "$REMOTE_BRANCHES" ]; then
    git push origin --delete $REMOTE_BRANCHES
fi

# Also delete any standalone remote 'release' branch
git push origin --delete release 2>/dev/null || true

# Prune deleted remote branches from local references
git fetch --prune

# Show remaining branches
echo "Remaining branches:"
git branch -a
```

## Safety notes:
- This will permanently delete branches that haven't been merged
- Make sure any important work has been merged to develop or main before running
- The command will fail gracefully if branches don't exist