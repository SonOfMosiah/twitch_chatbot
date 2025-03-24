mod welcome;

use anyhow::Result;
use std::collections::HashSet;
use std::path::Path;
use std::sync::RwLock;
use tokio::fs;
use tracing::{debug, info};

pub use welcome::WelcomeService;

/// User manager that tracks users who have interacted with the chat
pub struct UserManager {
    /// Set of user IDs who have already chatted at least once
    known_users: RwLock<HashSet<String>>,
    /// Path to file for persistence
    users_file_path: String,
}

impl UserManager {
    /// Create a new user manager
    ///
    /// # Arguments
    /// * `users_file_path` - Path to file for persisting known users
    ///
    /// # Returns
    /// A new UserManager instance
    pub fn new(users_file_path: &str) -> Self {
        UserManager {
            known_users: RwLock::new(HashSet::new()),
            users_file_path: users_file_path.to_string(),
        }
    }

    /// Load known users from file
    ///
    /// # Returns
    /// A Result indicating success or failure
    pub async fn load(&self) -> Result<()> {
        let path = Path::new(&self.users_file_path);

        // If the file doesn't exist, return without error
        if !path.exists() {
            debug!("Known users file doesn't exist yet. Starting with empty set.");
            return Ok(());
        }

        // Read and parse the file
        let content = fs::read_to_string(path).await?;
        let mut users = HashSet::new();

        for line in content.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                users.insert(trimmed.to_string());
            }
        }

        // Update the known users set
        {
            let mut known_users = self.known_users.write().unwrap();
            *known_users = users;
        }

        info!(
            "Loaded {} known users from {}",
            self.known_users.read().unwrap().len(),
            self.users_file_path
        );
        Ok(())
    }

    /// Save known users to file
    ///
    /// # Returns
    /// A Result indicating success or failure
    pub async fn save(&self) -> Result<()> {
        let users = {
            let known_users = self.known_users.read().unwrap();
            let mut users: Vec<String> = known_users.iter().cloned().collect();
            users.sort(); // Sort for consistent file output
            users
        };

        // Create the content as a sorted list of user IDs
        let content = users.join("\n");

        // Ensure the directory exists
        if let Some(parent) = Path::new(&self.users_file_path).parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).await?;
            }
        }

        // Write to the file
        fs::write(&self.users_file_path, content).await?;

        debug!(
            "Saved {} known users to {}",
            users.len(),
            self.users_file_path
        );
        Ok(())
    }

    /// Check if a user is a first-time chatter
    ///
    /// # Arguments
    /// * `user_id` - The Twitch user ID to check
    ///
    /// # Returns
    /// true if this is the first time seeing this user, false otherwise
    pub fn is_first_time_chatter(&self, user_id: &str) -> bool {
        let mut known_users = self.known_users.write().unwrap();

        if known_users.contains(user_id) {
            // User already known
            false
        } else {
            // New user! Add them to our known users set
            known_users.insert(user_id.to_string());
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_first_time_chatter() {
        let user_manager = UserManager::new("test_users.txt");

        // First time should be true
        assert!(user_manager.is_first_time_chatter("user1"));

        // Second time should be false
        assert!(!user_manager.is_first_time_chatter("user1"));

        // Different user should be true
        assert!(user_manager.is_first_time_chatter("user2"));
    }

    #[tokio::test]
    async fn test_load_save_users() -> Result<()> {
        // Create a temporary file
        let mut temp_file = NamedTempFile::new()?;
        let initial_users = "user1\nuser2\nuser3";
        write!(temp_file, "{}", initial_users)?;

        let temp_path = temp_file.path().to_str().unwrap().to_string();
        let user_manager = UserManager::new(&temp_path);

        // Load the users
        user_manager.load().await?;

        // Check if the users were loaded
        assert!(!user_manager.is_first_time_chatter("user1"));
        assert!(!user_manager.is_first_time_chatter("user2"));
        assert!(!user_manager.is_first_time_chatter("user3"));
        assert!(user_manager.is_first_time_chatter("user4"));

        // Save the users
        user_manager.save().await?;

        // Read the file and check its contents
        let content = fs::read_to_string(&temp_path).await?;
        let mut lines: Vec<&str> = content.lines().collect();
        lines.sort();

        assert_eq!(lines, vec!["user1", "user2", "user3", "user4"]);

        Ok(())
    }
}
