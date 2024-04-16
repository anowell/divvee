use std::{collections::BTreeMap, path::PathBuf};

use crate::{repo::ChangeInfo, Error, RepoDoc};
use gray_matter::{engine::YAML, Matter};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, sqlx::FromRow)]
pub struct Task {
    // id is not settable
    #[serde(default, skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[sqlx(skip)]
    pub labels: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[sqlx(skip)]
    pub props: BTreeMap<String, serde_yaml::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    // #[serde(skip_deserializing, skip_serializing_if = "Option::is_none")]
    // #[sqlx(skip)]
    // pub created: Option<ChangeInfo>,
    // #[serde(skip_deserializing, skip_serializing_if = "Option::is_none")]
    // #[sqlx(skip)]
    // pub updated: Option<ChangeInfo>,
}

impl Task {
    pub fn new(title: &str) -> Task {
        let title = title.into();
        Task {
            title,
            ..Default::default()
        }
    }

    pub fn id(&self) -> Option<String> {
        self.id.clone()
    }
}

impl RepoDoc for Task {
    fn parse_doc(s: &str, path: Option<PathBuf>) -> Result<Self, Error> {
        let matter = Matter::<YAML>::new();
        let res = matter.parse(s);
        let mut doc = res.data.unwrap().deserialize::<Task>()?;

        if !res.content.is_empty() {
            doc.description = Some(res.content);
        }
        doc.id = path.and_then(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()));
        Ok(doc)
    }

    fn to_doc_string(&self) -> String {
        let mut doc = self.clone();
        doc.id = None;
        let description = doc.description.take().unwrap_or_else(String::new);
        let yaml = serde_yaml::to_string(&doc).unwrap();
        format!("---\n{}\n---\n\n{}", yaml, description)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_new_task() {
        let mut doc = Task::new("hello");
        assert_eq!(doc.to_doc_string().trim(), "---\ntitle: hello\n\n---");
        doc.description = Some("description".to_string());
        assert_eq!(
            doc.to_doc_string().trim(),
            "---\ntitle: hello\n\n---\n\ndescription"
        );
    }

    #[test]
    fn test_task_parse() {
        let task = Task::parse_doc("---\ntitle: hello\n---", None).unwrap();
        assert_eq!(task.title, "hello");
        let task = Task::parse_doc("---\ntitle: hello\n---\n\ndescription", None).unwrap();
        assert_eq!(task.description.unwrap(), "description");
    }
}
