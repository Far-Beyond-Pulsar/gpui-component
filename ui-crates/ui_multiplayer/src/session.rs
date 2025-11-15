//! Session management functionality

use super::state::MultiplayerWindow;
use super::types::*;

impl MultiplayerWindow {
        pub(super) fn format_participants(&self, participants: &[String]) -> Vec<String> {
        let our_peer_id = match &self.current_peer_id {
            Some(id) => id,
            None => return participants.to_vec(),
        };

        let is_host = self.active_session.as_ref()
            .map(|s| participants.first() == Some(our_peer_id))
            .unwrap_or(false);

        participants.iter().map(|p| {
            if p == our_peer_id {
                if is_host {
                    "You (Host)".to_string()
                } else {
                    "You".to_string()
                }
            } else {
                p.clone()
            }
        }).collect()
    }

}
