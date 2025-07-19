# Fix Production Tasks Command

This command helps complete production deployment tasks systematically.

## Steps to execute:

1. **Check production tasks status**
   - Read the next documents in docs/ to identify uncompleted tasks (marked with [ ])
   - Select the next logical task to work on

2. **Create feature branch**
   - Create a new branch named: `feat/task-name` (e.g., `feat/cost-management`)
   - Switch to the new branch

3. **Create task documentation**
   - Create a new task file in `docs/tasks/` following the naming pattern:
     - Format: `XXX-task-description.md` (e.g., `033-cost-management.md`)
   - Document the task objectives and implementation plan

4. **Implement the task**
   - Work on the implementation
   - Run `npm run build` to verify no build errors
   - Run `npm run lint` to ensure code quality
   - Fix any issues that arise

5. **Update documentation**
   - Mark the task as completed [x] in the document you are working on
   - Update docs/PRD.md if the task affects project requirements
   - Reference the task file in the production tasks document

6. **Create pull request**
   - Commit all changes with descriptive commit messages
   - Push the branch to remote
   - Create a PR with:
     - Clear title describing what was fixed
     - Reference to the task documentation
     - Summary of changes made
   - go back on the develop branch 

## Important notes:
- Always check budget optimization tasks (section 13) as they directly impact the $30/month hosting goal
- Prioritize critical checklist items if any are uncompleted
- Ensure all changes maintain production-ready quality