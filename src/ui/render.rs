use colored::Colorize;
use crate::jj::types::{BookmarkSyncState, ChangeWithStatus};
use super::{IconSet, Theme};

pub struct Renderer {
    theme: &'static Theme,
    icons: &'static IconSet,
}

impl Renderer {
    pub fn new(theme: &'static Theme, icons: &'static IconSet) -> Self {
        Self { theme, icons }
    }
    
    /// Render the stack status
    pub fn render_stack(&self, changes: &[ChangeWithStatus], main_ref: &str) {
        let total = changes.len();

        let title = if total > 0 {
            format!("Your Stack ({} commits)", total)
        } else {
            "Your Stack".to_string()
        };

        // Buffer content lines so the box can stretch to the longest one
        let mut lines: Vec<String> = Vec::new();

        if changes.is_empty() {
            lines.push("  No changes in stack".to_string());
            lines.push(format!("  (All work is integrated into {})", main_ref));
        } else {
            for (i, item) in changes.iter().enumerate() {
                // Position: 1 is closest to trunk, total is the head
                let position = total - i;
                self.render_change(&mut lines, item, position, total);

                // Add spacing between changes (except for last)
                if i < changes.len() - 1 {
                    self.push_connection(&mut lines);
                }
            }
            self.push_connection(&mut lines);
        }
        self.push_main(&mut lines, main_ref);

        let inner_width = Self::inner_width_for(&lines, &title);

        println!();
        println!("{}", self.box_top_line(&title, inner_width));
        println!();
        for line in &lines {
            println!("{}", line);
        }
        println!();
        println!("{}", Self::box_bottom_line(inner_width));
        println!();

        // Print suggestions
        self.print_suggestions(changes);
    }

    fn render_change(&self, lines: &mut Vec<String>, item: &ChangeWithStatus, position: usize, total: usize) {
        let is_working = item.is_working;

        // Icon
        let icon = if is_working {
            self.icons.working
        } else {
            self.icons.change
        };

        let icon_colored = if is_working {
            icon.color(self.theme.mauve)
        } else {
            icon.color(self.theme.text)
        };

        // Position marker (e.g., "3/5")
        let position_marker = format!("{}/{}", position, total).color(self.theme.overlay);

        // Change ID (first 8 chars)
        let change_id = &item.change.change_id[..8.min(item.change.change_id.len())];
        let change_id_colored = change_id.color(self.theme.blue);

        // Description
        let description = item.change.description
            .lines()
            .next()
            .unwrap_or("(no description)")
            .color(self.theme.text);

        // Mark changes with no file modifications, like jj log does
        let empty_marker = if item.change.empty {
            format!("{} ", "(empty)".color(self.theme.overlay))
        } else {
            String::new()
        };

        // Main line with position
        lines.push(format!(
            "  {} {}  {}  {}{}",
            position_marker, icon_colored, change_id_colored, empty_marker, description
        ));

        // Bookmark line with sync state (if exists)
        if let Some(bookmark) = &item.bookmark {
            self.render_sync_state(lines, bookmark, &item.sync_state);
        }

        // Status line (aligned with bookmark line)
        if let Some(status_msg) = self.format_status(item) {
            lines.push(format!("         {}", status_msg));
        }
    }

    /// Render bookmark with sync state visualization
    fn render_sync_state(&self, lines: &mut Vec<String>, bookmark: &str, sync_state: &BookmarkSyncState) {
        let bookmark_icon = self.icons.bookmark.color(self.theme.teal);
        let bookmark_name = bookmark.color(self.theme.teal);

        match sync_state {
            BookmarkSyncState::NoBookmark => {
                // Shouldn't happen since we're called with a bookmark
            }
            BookmarkSyncState::LocalOnly => {
                lines.push(format!(
                    "         {} {} {}",
                    bookmark_icon,
                    bookmark_name,
                    "(local only)".color(self.theme.overlay)
                ));
            }
            BookmarkSyncState::Synced => {
                lines.push(format!(
                    "         {} {} {}",
                    bookmark_icon,
                    bookmark_name,
                    "✓".color(self.theme.green)
                ));
            }
            BookmarkSyncState::NeedsPush => {
                lines.push(format!(
                    "         {} {} {}",
                    bookmark_icon,
                    bookmark_name,
                    "↑ needs push".color(self.theme.yellow)
                ));
            }
            BookmarkSyncState::Ahead { count } => {
                // Local is ahead of remote
                lines.push(format!(
                    "         {} {} {} {}",
                    bookmark_icon,
                    bookmark_name,
                    format!("↑{}", count).color(self.theme.green),
                    "ahead".color(self.theme.overlay)
                ));
            }
            BookmarkSyncState::Behind { count } => {
                // Local is behind remote
                lines.push(format!(
                    "         {} {} {} {}",
                    bookmark_icon,
                    bookmark_name,
                    format!("↓{}", count).color(self.theme.yellow),
                    "behind".color(self.theme.overlay)
                ));
            }
            BookmarkSyncState::Diverged { local_ahead, remote_ahead, fork_point } => {
                // Show diverged state with fork visualization
                let fork_id = fork_point.as_deref().unwrap_or("???");

                // Fork visualization - the ○ fork point must align with ╭ and ╰:
                //                   ╭──●──●    local (+2)
                //       bookmark ───○ abc123
                //                   ╰──○──○    origin (+1) ⚠ diverged

                // Base indent for bookmark line (9 spaces to align with change_id)
                let base_indent = "         ";

                // Build the bookmark prefix: "{base_indent}{bookmark_icon} {bookmark_name} ───"
                let prefix = format!("{}{} {} ───", base_indent, self.icons.bookmark, bookmark);
                let prefix_width = console::measure_text_width(&prefix);

                // Fork arms (╭ and ╰) start at same column as the ○
                let fork_indent = " ".repeat(prefix_width);

                // Build chains: ──●──●──● for local, ──○──○──○ for remote
                let local_chain_dots: Vec<&str> = (0..*local_ahead).map(|_| "●").collect();
                let local_chain_str = local_chain_dots.join("──");
                let local_chain = format!("╭──{}    local (+{})", local_chain_str, local_ahead);
                lines.push(format!(
                    "{}{}",
                    fork_indent,
                    local_chain.color(self.theme.green)
                ));

                // Fork point with bookmark
                lines.push(format!(
                    "{}○ {}",
                    prefix.color(self.theme.teal),
                    fork_id.color(self.theme.overlay)
                ));

                // Remote branch (below fork point)
                let remote_chain_dots: Vec<&str> = (0..*remote_ahead).map(|_| "○").collect();
                let remote_chain_str = remote_chain_dots.join("──");
                let remote_chain = format!("╰──{}    origin (+{}) ⚠ diverged", remote_chain_str, remote_ahead);
                lines.push(format!(
                    "{}{}",
                    fork_indent,
                    remote_chain.color(self.theme.red)
                ));
            }
        }
    }

    fn format_status(&self, item: &ChangeWithStatus) -> Option<String> {
        if item.bookmark.is_none() && !item.is_working {
            Some(format!("{} ready to create PR", self.icons.lightbulb))
        } else {
            None
        }
    }
    
    fn push_connection(&self, lines: &mut Vec<String>) {
        // Align pipe with the icon position
        // Main line: "  {pos} {icon}  {id}  {desc}"
        // "  1/1 " = 6 chars, then icon
        lines.push(format!("      {}", self.icons.pipe.color(self.theme.overlay)));
    }

    fn push_main(&self, lines: &mut Vec<String>, main_ref: &str) {
        // Align with the icon position
        // Main line: "  {pos} {icon}  {id}  {desc}"
        // "  1/1 " = 6 chars, then icon
        lines.push(format!(
            "      {}  {}",
            self.icons.main.color(self.theme.blue),
            main_ref.color(self.theme.blue)
        ));
    }

    /// Inner width of the box: wide enough for the longest content line
    /// (plus a small right margin) and the title, with a sane floor.
    fn inner_width_for(lines: &[String], title: &str) -> usize {
        let content_width = lines
            .iter()
            .map(|l| console::measure_text_width(l))
            .max()
            .unwrap_or(0);
        let title_width = console::measure_text_width(title) + 2; // " title "
        (content_width + 2).max(title_width + 8).max(40)
    }

    fn box_top_line(&self, title: &str, inner_width: usize) -> String {
        let title_with_padding = format!(" {} ", title);
        let title_len = console::measure_text_width(&title_with_padding);
        let remaining = inner_width.saturating_sub(title_len);
        let left_padding = remaining / 2;
        let right_padding = remaining - left_padding;

        format!(
            "╭{}{}{}╮",
            "─".repeat(left_padding),
            title_with_padding.color(self.theme.text),
            "─".repeat(right_padding)
        )
    }

    fn box_bottom_line(inner_width: usize) -> String {
        format!("╰{}╯", "─".repeat(inner_width))
    }

    /// Command suggestions: flat, icon-free, left-aligned lines
    fn suggestion_lines(&self, changes: &[ChangeWithStatus]) -> Vec<String> {
        let mut suggestions = Vec::new();

        // Check if there are changes without bookmarks
        let needs_bookmark = changes.iter().any(|c| c.bookmark.is_none() && !c.is_working);
        if needs_bookmark {
            suggestions.push(format!(
                "  {}  {}",
                "jf push".color(self.theme.text),
                "push stack to GitHub".color(self.theme.overlay)
            ));
        }

        suggestions.push(format!(
            "  {}  {}",
            "jf pull".color(self.theme.text),
            "update from remote".color(self.theme.overlay)
        ));

        suggestions
    }

    fn print_suggestions(&self, changes: &[ChangeWithStatus]) {
        for suggestion in self.suggestion_lines(changes) {
            println!("{}", suggestion);
        }
        println!();
    }
    
    /// Render error message
    pub fn error(&self, message: &str) {
        eprintln!(
            "{} {}",
            self.icons.error.color(self.theme.red),
            message.color(self.theme.red)
        );
    }
    
    /// Render success message
    pub fn success(&self, message: &str) {
        println!(
            "{} {}",
            self.icons.pr_approved.color(self.theme.green),
            message.color(self.theme.green)
        );
    }
    
    /// Render info message
    pub fn info(&self, message: &str) {
        println!(
            "{} {}",
            self.icons.info.color(self.theme.blue),
            message
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::{get_icon_set, get_theme};

    #[test]
    fn box_lines_match_requested_width() {
        let r = Renderer::new(get_theme("default"), get_icon_set("unicode"));
        let top = r.box_top_line("Your Stack (2 commits)", 50);
        assert!(top.starts_with('╭'));
        assert!(top.ends_with('╮'));
        assert_eq!(console::measure_text_width(&top), 52);

        let bottom = Renderer::box_bottom_line(50);
        assert!(bottom.starts_with('╰'));
        assert!(bottom.ends_with('╯'));
        assert_eq!(console::measure_text_width(&bottom), 52);
    }

    #[test]
    fn box_stretches_to_longest_content_line() {
        let long_line = "  1/2 ○  wtwpxurp  Add TEMP Sentry token cleanup reminder to CLAUDE.md";
        let lines = vec!["  short".to_string(), long_line.to_string()];
        let inner = Renderer::inner_width_for(&lines, "Your Stack (2 commits)");
        assert!(
            inner >= console::measure_text_width(long_line) + 2,
            "box must cover the longest line plus margin (inner: {})",
            inner
        );
    }

    #[test]
    fn inner_width_has_floor_for_short_content() {
        let lines = vec!["  x".to_string()];
        let inner = Renderer::inner_width_for(&lines, "Your Stack");
        assert!(inner >= 40, "short content still gets a reasonable box");
    }

    #[test]
    fn suggestions_are_flat_and_left_aligned() {
        let r = Renderer::new(get_theme("default"), get_icon_set("unicode"));
        let lines = r.suggestion_lines(&[]);
        assert!(!lines.is_empty());
        for line in &lines {
            assert!(
                line.starts_with("  jf "),
                "commands must be left-aligned with no icon prefix: {:?}",
                line
            );
            assert!(!line.contains("Quick commands"));
        }
    }
}
