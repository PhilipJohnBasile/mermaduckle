# Commit and push local changes for Mermaduckle
# Run this locally if the automatic git steps fail in the agent environment.

git add -A
git commit -m "chore: apply automated changes (README rewrite, dev port, console snippet)"
git push

# Notes:
# - Ensure your git remote is accessible and you have proper credentials.
# - If the commit fails because there are no changes, remove the commit step.
