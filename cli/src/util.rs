use crate::config;
use anyhow::Result;

/// Helper that extracts team from ID
///
/// For convenience, also allows infers the team portion
/// from defaults, if not included explicity
pub fn team_and_id(id: &str) -> Result<(String, String)> {
    match id.split('-').next() {
        Some(prefix) => Ok((prefix.to_owned(), id.to_owned())),
        None => {
            let team = config::default_team()?;
            Ok((team.clone(), format!("{}-{}", team, id)))
        }
    }
}
