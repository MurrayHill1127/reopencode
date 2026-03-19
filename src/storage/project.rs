//! Project storage operations

use std::sync::Arc;

use crate::storage::{
    ProjectRecord, StorageBackend, StorageError, WorkspaceRecord,
    backend::{read, write},
};

/// Project creation input
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProjectCreateInput {
    pub worktree: String,
    pub vcs: Option<String>,
}

/// Project storage
pub struct ProjectStore {
    backend: Arc<dyn StorageBackend>,
}

impl ProjectStore {
    pub fn new(backend: Arc<dyn StorageBackend>) -> Self {
        Self { backend }
    }

    pub async fn create(&self, input: ProjectCreateInput) -> Result<ProjectRecord, StorageError> {
        let mut project = ProjectRecord::new(input.worktree);
        project.vcs = input.vcs;

        let key = ["project", &format!("{}.json", project.id)];
        write(&*self.backend, &key, &project).await?;

        Ok(project)
    }

    pub async fn get(&self, project_id: &str) -> Result<Option<ProjectRecord>, StorageError> {
        let key = ["project", &format!("{}.json", project_id)];
        read(&*self.backend, &key).await
    }

    pub async fn get_by_worktree(
        &self,
        worktree: &str,
    ) -> Result<Option<ProjectRecord>, StorageError> {
        let keys = self.backend.list(&["project"]).await?;

        for key in keys {
            if key.len() >= 2 {
                let key_refs: Vec<&str> = key.iter().map(String::as_str).collect();
                if let Some(project) = read::<ProjectRecord>(&*self.backend, &key_refs).await?
                    && project.worktree == worktree
                {
                    return Ok(Some(project));
                }
            }
        }

        Ok(None)
    }

    pub async fn list(&self) -> Result<Vec<ProjectRecord>, StorageError> {
        let mut projects = Vec::new();

        let keys = self.backend.list(&["project"]).await?;

        for key in keys {
            if key.len() >= 2 {
                let key_refs: Vec<&str> = key.iter().map(String::as_str).collect();
                if let Some(project) = read::<ProjectRecord>(&*self.backend, &key_refs).await? {
                    projects.push(project);
                }
            }
        }

        projects.sort_by(|a, b| b.time_updated.cmp(&a.time_updated));

        Ok(projects)
    }

    pub async fn update(&self, project: &ProjectRecord) -> Result<(), StorageError> {
        let key = ["project", &format!("{}.json", project.id)];
        write(&*self.backend, &key, project).await
    }

    /// Delete a project
    pub async fn remove(&self, project_id: &str) -> Result<(), StorageError> {
        let key = ["project", &format!("{}.json", project_id)];
        self.backend.remove(&key).await
    }

    /// Mark project as initialized
    pub async fn mark_initialized(&self, project_id: &str) -> Result<ProjectRecord, StorageError> {
        let mut project = self
            .get(project_id)
            .await?
            .ok_or_else(|| StorageError::NotFound(project_id.to_string()))?;

        project.time_initialized = Some(chrono::Utc::now().timestamp_millis());
        project.time_updated = chrono::Utc::now().timestamp_millis();

        self.update(&project).await?;

        Ok(project)
    }
}

/// Workspace creation input
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkspaceCreateInput {
    pub project_id: String,
    pub name: String,
    pub directory: String,
}

/// Workspace storage
pub struct WorkspaceStore {
    backend: Arc<dyn StorageBackend>,
}

impl WorkspaceStore {
    pub fn new(backend: Arc<dyn StorageBackend>) -> Self {
        Self { backend }
    }

    pub async fn create(
        &self,
        input: WorkspaceCreateInput,
    ) -> Result<WorkspaceRecord, StorageError> {
        let now = chrono::Utc::now().timestamp_millis();
        let workspace = WorkspaceRecord {
            id: format!(
                "ws_{}",
                &crate::util::generate_uuid().replace('-', "")[..12]
            ),
            project_id: input.project_id,
            name: input.name,
            directory: input.directory,
            time_created: now,
            time_updated: now,
        };

        let key = ["workspace", &format!("{}.json", workspace.id)];
        write(&*self.backend, &key, &workspace).await?;

        Ok(workspace)
    }

    pub async fn get(&self, workspace_id: &str) -> Result<Option<WorkspaceRecord>, StorageError> {
        let key = ["workspace", &format!("{}.json", workspace_id)];
        read(&*self.backend, &key).await
    }

    pub async fn list_for_project(
        &self,
        project_id: &str,
    ) -> Result<Vec<WorkspaceRecord>, StorageError> {
        let mut workspaces = Vec::new();

        let keys = self.backend.list(&["workspace"]).await?;

        for key in keys {
            if key.len() >= 2 {
                let key_refs: Vec<&str> = key.iter().map(String::as_str).collect();
                if let Some(workspace) = read::<WorkspaceRecord>(&*self.backend, &key_refs).await?
                    && workspace.project_id == project_id
                {
                    workspaces.push(workspace);
                }
            }
        }

        workspaces.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(workspaces)
    }

    pub async fn update(&self, workspace: &WorkspaceRecord) -> Result<(), StorageError> {
        let key = ["workspace", &format!("{}.json", workspace.id)];
        write(&*self.backend, &key, workspace).await
    }

    /// Delete a workspace
    pub async fn remove(&self, workspace_id: &str) -> Result<(), StorageError> {
        let key = ["workspace", &format!("{}.json", workspace_id)];
        self.backend.remove(&key).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::JsonBackend;
    use tempfile::TempDir;

    fn create_test_stores() -> (ProjectStore, WorkspaceStore, TempDir) {
        let temp = TempDir::new().unwrap();
        let backend = Arc::new(JsonBackend::new(temp.path()).unwrap());

        (
            ProjectStore::new(backend.clone()),
            WorkspaceStore::new(backend),
            temp,
        )
    }

    #[tokio::test]
    async fn test_project_create() {
        let (store, _, _temp) = create_test_stores();

        let project = store
            .create(ProjectCreateInput {
                worktree: "/home/user/project".to_string(),
                vcs: Some("git".to_string()),
            })
            .await
            .unwrap();

        assert!(project.id.starts_with("proj_"));
        assert_eq!(project.worktree, "/home/user/project");
        assert_eq!(project.vcs, Some("git".to_string()));
    }

    #[tokio::test]
    async fn test_project_get() {
        let (store, _, _temp) = create_test_stores();

        let created = store
            .create(ProjectCreateInput {
                worktree: "/test/project".to_string(),
                vcs: None,
            })
            .await
            .unwrap();

        let loaded = store.get(&created.id).await.unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().id, created.id);
    }

    #[tokio::test]
    async fn test_project_get_by_worktree() {
        let (store, _, _temp) = create_test_stores();

        let worktree = "/unique/worktree/path";
        let created = store
            .create(ProjectCreateInput {
                worktree: worktree.to_string(),
                vcs: None,
            })
            .await
            .unwrap();

        let found = store.get_by_worktree(worktree).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, created.id);
    }

    #[tokio::test]
    async fn test_project_list() {
        let (store, _, _temp) = create_test_stores();

        store
            .create(ProjectCreateInput {
                worktree: "/project1".to_string(),
                vcs: None,
            })
            .await
            .unwrap();

        store
            .create(ProjectCreateInput {
                worktree: "/project2".to_string(),
                vcs: None,
            })
            .await
            .unwrap();

        let projects = store.list().await.unwrap();
        assert_eq!(projects.len(), 2);
    }

    #[tokio::test]
    async fn test_project_remove() {
        let (store, _, _temp) = create_test_stores();

        let project = store
            .create(ProjectCreateInput {
                worktree: "/test".to_string(),
                vcs: None,
            })
            .await
            .unwrap();

        store.remove(&project.id).await.unwrap();

        let loaded = store.get(&project.id).await.unwrap();
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn test_workspace_create() {
        let (_, store, _temp) = create_test_stores();

        let workspace = store
            .create(WorkspaceCreateInput {
                project_id: "proj_test".to_string(),
                name: "Development".to_string(),
                directory: "/home/user/project/dev".to_string(),
            })
            .await
            .unwrap();

        assert!(workspace.id.starts_with("ws_"));
        assert_eq!(workspace.name, "Development");
        assert_eq!(workspace.project_id, "proj_test");
    }

    #[tokio::test]
    async fn test_workspace_list_for_project() {
        let (_, store, _temp) = create_test_stores();

        let project_id = "proj_test";

        store
            .create(WorkspaceCreateInput {
                project_id: project_id.to_string(),
                name: "Dev".to_string(),
                directory: "/dev".to_string(),
            })
            .await
            .unwrap();

        store
            .create(WorkspaceCreateInput {
                project_id: project_id.to_string(),
                name: "Staging".to_string(),
                directory: "/staging".to_string(),
            })
            .await
            .unwrap();

        store
            .create(WorkspaceCreateInput {
                project_id: "proj_other".to_string(),
                name: "Other".to_string(),
                directory: "/other".to_string(),
            })
            .await
            .unwrap();

        let workspaces = store.list_for_project(project_id).await.unwrap();
        assert_eq!(workspaces.len(), 2);
    }
}
