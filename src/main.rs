use rand::seq::SliceRandom;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

type StudentId = String;

#[derive(Debug, Clone)]
struct Group {
    members: Vec<StudentId>,
}

impl Group {
    fn new() -> Self {
        Group {
            members: Vec::new(),
        }
    }

    fn add_member(&mut self, student_id: StudentId) {
        if self.members.len() < 3 {
            self.members.push(student_id);
        }
    }

    fn is_full(&self) -> bool {
        self.members.len() >= 3
    }
}

fn read_student_ids(running: Arc<AtomicBool>) -> Vec<Group> {
    let mut groups = Vec::new();
    let mut current_group = Group::new();

    println!("学籍番号を入力してください (3人ごとにグループになります):");
    println!("  - Ctrl+D (Unix/Mac) または Ctrl+Z+Enter (Windows): 現在のグループを終了して次のグループへ");
    println!("  - Ctrl+C: プログラムを終了");
    println!();

    let mut group_number = 1;
    println!("=== グループ {} の入力 ===", group_number);

    // Check if stdin is a TTY (interactive terminal)
    let is_tty = if cfg!(unix) {
        use std::os::unix::io::AsRawFd;
        unsafe { libc::isatty(io::stdin().as_raw_fd()) == 1 }
    } else {
        // On Windows, assume interactive for now
        true
    };

    loop {
        // Check if Ctrl+C was pressed
        if !running.load(Ordering::SeqCst) {
            // Save current group if it has members before exiting
            if !current_group.members.is_empty() {
                groups.push(current_group.clone());
            }
            break;
        }

        // Read input - use /dev/tty only in interactive mode on Unix
        let reader: Box<dyn BufRead> = if is_tty && cfg!(unix) {
            File::open("/dev/tty")
                .map(|f| Box::new(BufReader::new(f)) as Box<dyn BufRead>)
                .unwrap_or_else(|_| Box::new(BufReader::new(io::stdin())))
        } else {
            Box::new(BufReader::new(io::stdin()))
        };

        let mut eof_encountered = false;
        for line in reader.lines() {
            // Check if Ctrl+C was pressed
            if !running.load(Ordering::SeqCst) {
                if !current_group.members.is_empty() {
                    groups.push(current_group.clone());
                }
                return groups;
            }

            match line {
                Ok(student_id) => {
                    let student_id = student_id.trim().to_string();
                    if !student_id.is_empty() {
                        current_group.add_member(student_id.clone());
                        println!("  追加: {}", student_id);

                        if current_group.is_full() {
                            println!("  ✓ グループ {} が完成しました (3人)", group_number);
                            groups.push(current_group.clone());
                            current_group = Group::new();
                            group_number += 1;
                            println!("\n=== グループ {} の入力 ===", group_number);
                        }
                    }
                }
                Err(_) => {
                    eof_encountered = true;
                    break;
                }
            }
        }

        // EOF was encountered
        if !eof_encountered {
            // lines iterator ended naturally (EOF)
            eof_encountered = true;
        }

        if eof_encountered {
            // Save current group if it has members
            if !current_group.members.is_empty() {
                println!(
                    "  ✓ グループ {} を保存しました ({} 人)",
                    group_number,
                    current_group.members.len()
                );
                groups.push(current_group.clone());
                current_group = Group::new();
                group_number += 1;

                // Only continue for multiple groups if we're in interactive TTY mode with /dev/tty
                if is_tty && cfg!(unix) && File::open("/dev/tty").is_ok() {
                    println!("\n=== グループ {} の入力 ===", group_number);
                    // Continue loop to read next group
                    continue;
                }
            }

            // Exit the loop if not in interactive mode
            break;
        }
    }

    // Save the current group if it has any members
    if !current_group.members.is_empty() {
        groups.push(current_group);
    }

    groups
}

fn reorganize_incomplete_groups(groups: Vec<Group>) -> Vec<Group> {
    let mut final_groups = Vec::new();
    let mut incomplete_members = Vec::new();

    // Separate complete and incomplete groups (requirements 3 and 4)
    for group in groups {
        if group.is_full() {
            // Requirement 3: Don't modify groups with 3 members
            final_groups.push(group);
        } else {
            // Collect members from incomplete groups
            incomplete_members.extend(group.members);
        }
    }

    // Requirement 4: Randomly combine incomplete group members
    let mut rng = rand::thread_rng();
    incomplete_members.shuffle(&mut rng);

    // Create new 3-person groups from the shuffled members
    let mut new_group = Group::new();
    for member in incomplete_members {
        new_group.add_member(member);
        if new_group.is_full() {
            final_groups.push(new_group);
            new_group = Group::new();
        }
    }

    // Requirement 5: Allow 2-person groups if total isn't divisible by 3
    if !new_group.members.is_empty() {
        final_groups.push(new_group);
    }

    final_groups
}

fn print_groups(groups: &[Group]) {
    println!("\n=== グループ分け結果 ===");
    for (i, group) in groups.iter().enumerate() {
        println!("グループ {}: {} 人", i + 1, group.members.len());
        for member in &group.members {
            println!("  - {}", member);
        }
    }
    println!("\n合計: {} グループ", groups.len());
}

fn main() {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    // Set up Ctrl+C handler
    ctrlc::set_handler(move || {
        println!("\n\nCtrl+C が押されました。プログラムを終了します...");
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    let groups = read_student_ids(running);

    if groups.is_empty() {
        println!("\n入力されたデータがありません。");
        return;
    }

    let final_groups = reorganize_incomplete_groups(groups);
    print_groups(&final_groups);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group_creation() {
        let mut group = Group::new();
        assert!(!group.is_full());

        group.add_member("S001".to_string());
        assert!(!group.is_full());

        group.add_member("S002".to_string());
        assert!(!group.is_full());

        group.add_member("S003".to_string());
        assert!(group.is_full());

        // Try to add a 4th member (should not be added)
        group.add_member("S004".to_string());
        assert_eq!(group.members.len(), 3);
    }

    #[test]
    fn test_reorganize_with_complete_groups() {
        let mut group1 = Group::new();
        group1.add_member("S001".to_string());
        group1.add_member("S002".to_string());
        group1.add_member("S003".to_string());

        let mut group2 = Group::new();
        group2.add_member("S004".to_string());
        group2.add_member("S005".to_string());
        group2.add_member("S006".to_string());

        let groups = vec![group1, group2];
        let result = reorganize_incomplete_groups(groups);

        assert_eq!(result.len(), 2);
        assert!(result[0].is_full());
        assert!(result[1].is_full());
    }

    #[test]
    fn test_reorganize_with_incomplete_groups() {
        let mut group1 = Group::new();
        group1.add_member("S001".to_string());
        group1.add_member("S002".to_string());

        let mut group2 = Group::new();
        group2.add_member("S003".to_string());
        group2.add_member("S004".to_string());

        let groups = vec![group1, group2];
        let result = reorganize_incomplete_groups(groups);

        // 4 members should form 1 complete group and 1 incomplete group with 1 member
        let total_members: usize = result.iter().map(|g| g.members.len()).sum();
        assert_eq!(total_members, 4);
    }

    #[test]
    fn test_reorganize_allows_two_person_groups() {
        let mut group1 = Group::new();
        group1.add_member("S001".to_string());

        let mut group2 = Group::new();
        group2.add_member("S002".to_string());

        let groups = vec![group1, group2];
        let result = reorganize_incomplete_groups(groups);

        // 2 members should form 1 group with 2 members
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].members.len(), 2);
    }

    #[test]
    fn test_mixed_complete_and_incomplete_groups() {
        let mut group1 = Group::new();
        group1.add_member("S001".to_string());
        group1.add_member("S002".to_string());
        group1.add_member("S003".to_string());

        let mut group2 = Group::new();
        group2.add_member("S004".to_string());

        let mut group3 = Group::new();
        group3.add_member("S005".to_string());

        let groups = vec![group1, group2, group3];
        let result = reorganize_incomplete_groups(groups);

        // Should have 1 complete group (unchanged) + 1 group with 2 members
        assert_eq!(result.len(), 2);
        let complete_groups = result.iter().filter(|g| g.is_full()).count();
        assert_eq!(complete_groups, 1);
    }

    #[test]
    fn test_atomic_flag_behavior() {
        // Test that the atomic flag works correctly
        let running = Arc::new(AtomicBool::new(true));
        assert!(running.load(Ordering::SeqCst));

        running.store(false, Ordering::SeqCst);
        assert!(!running.load(Ordering::SeqCst));

        running.store(true, Ordering::SeqCst);
        assert!(running.load(Ordering::SeqCst));
    }
}
