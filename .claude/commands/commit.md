# Commit and PR Workflow

When this command is executed:

1. **Update documentation** before committing:
   - Update PRD.md with any relevant changes from the current task
   - Update any other relevant documentation based on the work done
2. **Stage all changes** - Add all modified and new files to git (including updated docs)
3. **Create or use appropriate branch**:
   - If currently on `develop` or `main`: Create a new feature or fix or chore branch
   - If already on a feature or fix or chore branch: Use the current branch
4. **Commit changes** with a descriptive message
5. **Push to remote** repository
6. **Create a Pull Request** targeting the `develop` branch
7. **Go back on `develop` branch**

This ensures all work is properly reviewed before merging into the main development branch and documentation stays up-to-date.