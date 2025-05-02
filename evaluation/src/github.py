import logging
import requests

logger = logging.getLogger(__name__)


def trigger_workflow(repository: str, workflow_id: str, branch: str, github_token: str) -> bool:
    """
    Triggers a workflow dispatch event for the specified repository and workflow_id.

    Args:
        repository: The GitHub repository in format 'owner/repo'
        branch: The branch to trigger the workflow on
        workflow_id: The workflow_id to trigger
        github_token: The GitHub PAT token to use for authentication

    Returns:
        bool: True if successful, False otherwise
    """
    if not repository or not repository.strip():
        logger.error("No repository provided")
        return False

    if not repository.count('/') == 1:
        logger.error("Invalid repository format. Expected 'owner/repo'")
        return False

    if not branch or not branch.strip():
        logger.error("No branch provided")
        return False

    if not workflow_id or not workflow_id.strip():
        logger.error("No workflow_id provided")
        return False

    if not github_token:
        logger.error("No GitHub token provided")
        return False

    url = f"https://api.github.com/repos/{repository}/actions/workflows/{workflow_id}.yml/dispatches"
    headers = {
        "Accept": "application/vnd.github+json",
        "Authorization": f"Bearer {github_token}",
    }

    data = {
        "ref": branch,
    }

    try:
        response = requests.post(url, headers=headers, json=data)
        response.raise_for_status()
        return True
    except requests.exceptions.RequestException as e:
        logger.error(f"Failed to trigger workflow: {str(e)}")
        return False
