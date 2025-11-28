use rand::seq::SliceRandom;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

type StudentId = String;

// Convert a group index (0-based) to a letter (A, B, C, ...)
fn group_index_to_letter(index: usize) -> String {
    let alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    if index < alphabet.len() {
        alphabet.chars().nth(index).unwrap().to_string()
    } else {
        // For indices beyond Z, use AA, AB, AC, etc.
        let first = (index / 26) - 1;
        let second = index % 26;
        format!(
            "{}{}",
            alphabet.chars().nth(first).unwrap(),
            alphabet.chars().nth(second).unwrap()
        )
    }
}

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

fn read_student_ids(running: Arc<AtomicBool>) -> (Vec<Group>, bool) {
    let mut groups = Vec::new();
    let mut current_group = Group::new();

    // Check if stdin is a TTY (interactive terminal)
    #[cfg(unix)]
    let is_tty = {
        use std::os::unix::io::AsRawFd;
        unsafe { libc::isatty(io::stdin().as_raw_fd()) == 1 }
    };

    #[cfg(windows)]
    let is_tty = {
        use std::os::windows::io::AsRawHandle;
        let handle = io::stdin().as_raw_handle();
        let mut mode: u32 = 0;
        // GetConsoleMode returns 0 if the handle is not a console
        unsafe {
            #[link(name = "kernel32")]
            extern "system" {
                fn GetConsoleMode(hConsoleHandle: *mut std::ffi::c_void, lpMode: *mut u32) -> i32;
            }
            GetConsoleMode(handle as *mut std::ffi::c_void, &mut mode) != 0
        }
    };

    #[cfg(not(any(unix, windows)))]
    let is_tty = true;

    // In batch mode (non-interactive), blank lines separate groups
    let batch_mode = !is_tty;

    if !batch_mode {
        println!("学籍番号を入力してください (3人ごとにグループになります):");
        println!("  - Ctrl+D (Unix/Mac) または Ctrl+Z+Enter (Windows): 現在のグループを終了して次のグループへ");
        println!("  - Ctrl+C: プログラムを終了");
        println!("  - 'delete:学籍番号' と入力すると、その学籍番号を削除できます（例: delete:S001）");
        println!();
    }

    let mut group_index = 0;
    if !batch_mode {
        println!(
            "=== グループ {} の入力 ===",
            group_index_to_letter(group_index)
        );
    }

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
                return (groups, batch_mode);
            }

            match line {
                Ok(student_id) => {
                    let student_id = student_id.trim().to_string();
                    
                    // In batch mode, empty lines separate groups
                    if student_id.is_empty() {
                        if batch_mode && !current_group.members.is_empty() {
                            // Save current group and start a new one
                            groups.push(current_group.clone());
                            current_group = Group::new();
                            group_index += 1;
                        }
                        continue;
                    }
                    
                    // Check if this is a delete command
                    if student_id.to_lowercase().starts_with("delete:") {
                        let id_to_delete = student_id[7..].trim().to_string();
                        
                        // Try to delete from current group first
                        let mut deleted = false;
                        if let Some(pos) = current_group.members.iter().position(|x| x == &id_to_delete) {
                            current_group.members.remove(pos);
                            println!("  ✓ 削除しました: {} (現在のグループから)", id_to_delete);
                            deleted = true;
                        }
                        
                        // If not found in current group, search in completed groups
                        if !deleted {
                            for (i, group) in groups.iter_mut().enumerate() {
                                if let Some(pos) = group.members.iter().position(|x| x == &id_to_delete) {
                                    group.members.remove(pos);
                                    println!(
                                        "  ✓ 削除しました: {} (グループ {} から)",
                                        id_to_delete,
                                        group_index_to_letter(i)
                                    );
                                    deleted = true;
                                    break;
                                }
                            }
                        }
                        
                        if !deleted {
                            println!("  ✗ エラー: {} は見つかりませんでした", id_to_delete);
                        }
                    } else {
                        // Normal student ID addition
                        if batch_mode {
                            // In batch mode, groups are unlimited in size (no 3-person limit)
                            current_group.members.push(student_id.clone());
                        } else {
                            // In interactive mode, use the 3-person limit
                            current_group.add_member(student_id.clone());
                            println!("  追加: {}", student_id);

                            if current_group.is_full() {
                                println!(
                                    "  ✓ グループ {} が完成しました (3人)",
                                    group_index_to_letter(group_index)
                                );
                                groups.push(current_group.clone());
                                current_group = Group::new();
                                group_index += 1;
                                println!(
                                    "\n=== グループ {} の入力 ===",
                                    group_index_to_letter(group_index)
                                );
                            }
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
                if !batch_mode {
                    println!(
                        "  ✓ グループ {} を保存しました ({} 人)",
                        group_index_to_letter(group_index),
                        current_group.members.len()
                    );
                }
                groups.push(current_group.clone());
                current_group = Group::new();
                group_index += 1;

                // Only continue for multiple groups if we're in interactive TTY mode with /dev/tty
                if is_tty && cfg!(unix) && File::open("/dev/tty").is_ok() {
                    println!(
                        "\n=== グループ {} の入力 ===",
                        group_index_to_letter(group_index)
                    );
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

    (groups, batch_mode)
}

/// Helper function to split a list of members into groups of 2-3 people
fn split_into_small_groups(members: Vec<StudentId>) -> Vec<Group> {
    let mut result: Vec<Group> = Vec::new();
    let n = members.len();
    
    if n == 0 {
        return result;
    }
    
    // Edge case: single member should not create a singleton
    // This shouldn't happen in normal use since we only split groups > 3
    if n == 1 {
        let mut group = Group::new();
        group.members.push(members[0].clone());
        result.push(group);
        return result;
    }
    
    let mut idx = 0;
    while idx < n {
        let remaining = n - idx;
        
        let group_size = if remaining >= 3 {
            if remaining == 4 {
                // 4 -> 2 + 2
                2
            } else {
                3
            }
        } else {
            // 2 remaining (1 is not possible when n >= 2 due to the algorithm)
            remaining
        };
        
        let mut new_group = Group::new();
        for i in 0..group_size {
            new_group.members.push(members[idx + i].clone());
        }
        idx += group_size;
        result.push(new_group);
    }
    
    result
}

/// Create groups of 2-3 people from a list of singletons
/// Uses the same logic as split_into_small_groups but is clearer about intent
fn group_singletons(singletons: &mut Vec<StudentId>) -> Vec<Group> {
    let mut groups: Vec<Group> = Vec::new();
    
    while singletons.len() >= 2 {
        // When we have exactly 4, split into 2+2 to avoid creating 3+1 (which would leave a singleton)
        let take_count = if singletons.len() == 4 { 2 } else { std::cmp::min(3, singletons.len()) };
        let mut new_group = Group::new();
        new_group.members = singletons.drain(0..take_count).collect();
        groups.push(new_group);
    }
    
    groups
}

/// Reorganize groups from batch mode - merge singletons and split groups larger than 3 into 2-3 person groups
/// Preserves 2-person and 3-person groups as-is, only merging singletons when the group won't exceed 3 members
fn reorganize_batch_groups(groups: Vec<Group>) -> Vec<Group> {
    if groups.is_empty() {
        return groups;
    }

    // First pass: split groups that are originally larger than 3 into 2-3 person groups
    // This preserves original 2 and 3 person group structures
    let mut split_groups: Vec<Group> = Vec::new();
    
    for group in groups {
        if group.members.len() <= 3 {
            split_groups.push(group);
        } else {
            // Split into 2-3 person groups
            let small_groups = split_into_small_groups(group.members);
            split_groups.extend(small_groups);
        }
    }
    
    // Second pass: merge singletons with adjacent groups (only if they won't exceed 3)
    let mut result_groups: Vec<Group> = Vec::new();
    let mut pending_singletons: Vec<StudentId> = Vec::new();
    
    for group in split_groups {
        if group.members.len() == 1 {
            // Collect singleton member
            pending_singletons.extend(group.members);
            
            // If we have 2 or 3 singletons, create a group immediately
            if pending_singletons.len() >= 2 && pending_singletons.len() <= 3 {
                let mut new_group = Group::new();
                new_group.members = pending_singletons.drain(..).collect();
                result_groups.push(new_group);
            }
        } else {
            // Try to add pending singletons to previous group first
            if !pending_singletons.is_empty() {
                if let Some(last_group) = result_groups.last_mut() {
                    let slots = 3 - last_group.members.len();
                    let take = std::cmp::min(slots, pending_singletons.len());
                    if take > 0 {
                        last_group.members.extend(pending_singletons.drain(0..take));
                    }
                }
                
                // Create groups from remaining singletons
                result_groups.extend(group_singletons(&mut pending_singletons));
            }
            
            // Add current group, possibly merging remaining singleton
            if pending_singletons.len() == 1 && group.members.len() < 3 {
                let mut merged_group = group;
                merged_group.members.insert(0, pending_singletons.drain(..).next().unwrap());
                result_groups.push(merged_group);
            } else {
                result_groups.push(group);
            }
        }
    }
    
    // Handle remaining singletons at the end
    if !pending_singletons.is_empty() {
        // Try to merge with the last group if it has space
        if let Some(last_group) = result_groups.last_mut() {
            let slots = 3 - last_group.members.len();
            let take = std::cmp::min(slots, pending_singletons.len());
            if take > 0 {
                last_group.members.extend(pending_singletons.drain(0..take));
            }
        }
        
        // Create groups from remaining singletons
        result_groups.extend(group_singletons(&mut pending_singletons));
        
        // If there's still a single person left, we need to add them somewhere
        if pending_singletons.len() == 1 {
            if let Some(last_group) = result_groups.last_mut() {
                // Add to last group even if it exceeds 3 (we'll need to split later)
                last_group.members.extend(pending_singletons.drain(..));
            } else {
                // No groups at all - this is an edge case (only 1 person total)
                let mut new_group = Group::new();
                new_group.members = pending_singletons;
                result_groups.push(new_group);
            }
        }
    }
    
    // Final pass: ensure no groups exceed 3 members
    let mut final_groups: Vec<Group> = Vec::new();
    for group in result_groups {
        if group.members.len() <= 3 {
            final_groups.push(group);
        } else {
            let small_groups = split_into_small_groups(group.members);
            final_groups.extend(small_groups);
        }
    }
    
    final_groups
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

    let n = incomplete_members.len();
    
    // NEW REQUIREMENT: Never create single-person groups
    // UPDATED: Prefer groups of 2-3 people, not 4
    if n == 1 {
        // If we have exactly 1 incomplete member and at least one complete group,
        // we should avoid creating a 4-person group
        if !final_groups.is_empty() {
            if let Some(last_group) = final_groups.last_mut() {
                // Take 1 member from the last complete group and pair with the singleton
                // to create two 2-person groups instead of one 4-person group
                if last_group.members.len() == 3 {
                    let member_from_last = last_group.members.pop().unwrap();
                    let mut new_group = Group::new();
                    new_group.members.push(member_from_last);
                    new_group.members.push(incomplete_members.into_iter().next().unwrap());
                    final_groups.push(new_group);
                } else {
                    // If the last group doesn't have 3 members, just add to it
                    last_group.members.push(incomplete_members.into_iter().next().unwrap());
                }
            }
        } else {
            // If we have no complete groups and only 1 member total, we cannot form valid groups
            // This case should be handled by the caller
            println!("警告: 1人だけではグループを作成できません。最低2人必要です。");
        }
        return final_groups;
    }

    // Create new groups from the shuffled members
    // Strategy: create groups of 3, but ensure the last group has at least 2 members
    let mut idx = 0;
    while idx < n {
        let remaining = n - idx;
        
        if remaining >= 3 {
            // If we can make a group of 3 or more
            let group_size = if remaining == 4 {
                // Special case: 4 remaining should be split into 2+2, not 3+1
                2
            } else {
                3
            };
            
            let mut new_group = Group::new();
            for _ in 0..group_size {
                if idx < n {
                    new_group.members.push(incomplete_members[idx].clone());
                    idx += 1;
                }
            }
            final_groups.push(new_group);
        } else {
            // remaining is 1 or 2
            if remaining == 2 {
                // Make a 2-person group
                let mut new_group = Group::new();
                new_group.members.push(incomplete_members[idx].clone());
                new_group.members.push(incomplete_members[idx + 1].clone());
                final_groups.push(new_group);
                idx += 2;
            } else {
                // remaining == 1: add to the last group instead of creating a singleton
                if let Some(last_group) = final_groups.last_mut() {
                    last_group.members.push(incomplete_members[idx].clone());
                    idx += 1;
                }
            }
        }
    }

    final_groups
}

fn print_groups(groups: &[Group]) {
    println!("\n=== グループ分け結果 ===");
    for (i, group) in groups.iter().enumerate() {
        println!(
            "グループ {}: {} 人",
            group_index_to_letter(i),
            group.members.len()
        );
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
        println!("\n\nCtrl+C が押されました。プログラムを終了します...グループ分けの結果を表示するには、Enterを押してください。");
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    let (groups, batch_mode) = read_student_ids(running);

    if groups.is_empty() {
        println!("\n入力されたデータがありません。");
        return;
    }

    // Use different reorganization logic based on mode
    let final_groups = if batch_mode {
        // Batch mode: preserve group structure, only merge singletons
        reorganize_batch_groups(groups)
    } else {
        // Interactive mode: reorganize incomplete groups
        reorganize_incomplete_groups(groups)
    };
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

        // 4 members should form 2 groups of 2 (not 3+1 which would create a singleton)
        let total_members: usize = result.iter().map(|g| g.members.len()).sum();
        assert_eq!(total_members, 4);
        assert_eq!(result.len(), 2);
        // Verify no single-person groups
        for group in &result {
            assert!(group.members.len() >= 2, "No group should have less than 2 members");
        }
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

        // Should have 1 complete group (unchanged) + 1 group with 2 members (no singletons)
        assert_eq!(result.len(), 2);
        let complete_groups = result.iter().filter(|g| g.is_full()).count();
        assert_eq!(complete_groups, 1);
        // Verify no single-person groups
        for group in &result {
            assert!(group.members.len() >= 2, "No group should have less than 2 members");
        }
    }

    #[test]
    fn test_no_single_person_groups_with_seven_students() {
        // Test with 7 students (would normally be 3+3+1)
        let groups = vec![
            {
                let mut g = Group::new();
                g.add_member("S001".to_string());
                g.add_member("S002".to_string());
                g
            },
            {
                let mut g = Group::new();
                g.add_member("S003".to_string());
                g.add_member("S004".to_string());
                g
            },
            {
                let mut g = Group::new();
                g.add_member("S005".to_string());
                g.add_member("S006".to_string());
                g
            },
            {
                let mut g = Group::new();
                g.add_member("S007".to_string());
                g
            },
        ];
        
        let result = reorganize_incomplete_groups(groups);

        // Should not have any single-person groups
        for group in &result {
            assert!(group.members.len() >= 2, "No group should have less than 2 members");
        }
        
        // Total should still be 7
        let total: usize = result.iter().map(|g| g.members.len()).sum();
        assert_eq!(total, 7);
    }

    #[test]
    fn test_no_single_person_groups_with_ten_students() {
        // Test with 10 students all incomplete (would normally be 3+3+3+1)
        let groups = vec![
            {
                let mut g = Group::new();
                g.add_member("S001".to_string());
                g.add_member("S002".to_string());
                g
            },
            {
                let mut g = Group::new();
                g.add_member("S003".to_string());
                g.add_member("S004".to_string());
                g
            },
            {
                let mut g = Group::new();
                g.add_member("S005".to_string());
                g.add_member("S006".to_string());
                g
            },
            {
                let mut g = Group::new();
                g.add_member("S007".to_string());
                g.add_member("S008".to_string());
                g
            },
            {
                let mut g = Group::new();
                g.add_member("S009".to_string());
                g.add_member("S010".to_string());
                g
            },
        ];

        let result = reorganize_incomplete_groups(groups);

        // Should not have any single-person groups
        for group in &result {
            assert!(group.members.len() >= 2, "No group should have less than 2 members");
        }
        
        // Total should still be 10
        let total: usize = result.iter().map(|g| g.members.len()).sum();
        assert_eq!(total, 10);
    }

    #[test]
    fn test_single_student_with_complete_group() {
        // Test 1 incomplete student with 1 complete group
        let mut complete_group = Group::new();
        complete_group.add_member("S001".to_string());
        complete_group.add_member("S002".to_string());
        complete_group.add_member("S003".to_string());

        let mut single_group = Group::new();
        single_group.add_member("S004".to_string());

        let groups = vec![complete_group, single_group];
        let result = reorganize_incomplete_groups(groups);

        // Should create two 2-person groups instead of one 4-person group
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].members.len(), 2);
        assert_eq!(result[1].members.len(), 2);
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

    #[test]
    fn test_reorganize_batch_groups_preserves_groups() {
        // Test that batch groups are preserved as-is
        let mut group1 = Group::new();
        group1.members = vec!["A".to_string(), "B".to_string(), "C".to_string()];

        let mut group2 = Group::new();
        group2.members = vec!["D".to_string(), "E".to_string()];

        let groups = vec![group1, group2];
        let result = reorganize_batch_groups(groups);

        // Groups should be preserved
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].members.len(), 3);
        assert_eq!(result[1].members.len(), 2);
    }

    #[test]
    fn test_reorganize_batch_groups_merges_singleton() {
        // Test that singletons are merged with previous group
        let mut group1 = Group::new();
        group1.members = vec!["A".to_string(), "B".to_string()];

        let mut group2 = Group::new();
        group2.members = vec!["C".to_string()]; // singleton

        let groups = vec![group1, group2];
        let result = reorganize_batch_groups(groups);

        // Singleton should be merged with previous group
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].members.len(), 3);
        assert!(result[0].members.contains(&"A".to_string()));
        assert!(result[0].members.contains(&"B".to_string()));
        assert!(result[0].members.contains(&"C".to_string()));
    }

    #[test]
    fn test_reorganize_batch_groups_singleton_at_start() {
        // Test that singleton at the start is merged with next group
        let mut group1 = Group::new();
        group1.members = vec!["A".to_string()]; // singleton at start

        let mut group2 = Group::new();
        group2.members = vec!["B".to_string(), "C".to_string()];

        let groups = vec![group1, group2];
        let result = reorganize_batch_groups(groups);

        // Singleton at start should be merged with next group
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].members.len(), 3);
    }

    #[test]
    fn test_reorganize_batch_groups_multiple_singletons() {
        // Test multiple singletons are grouped together, preserving existing 2-person group
        let mut group1 = Group::new();
        group1.members = vec!["A".to_string(), "B".to_string()];

        let mut group2 = Group::new();
        group2.members = vec!["C".to_string()]; // singleton

        let mut group3 = Group::new();
        group3.members = vec!["D".to_string()]; // singleton

        let groups = vec![group1, group2, group3];
        let result = reorganize_batch_groups(groups);

        // Original 2-person group [A, B] should be preserved, singletons [C, D] form new group
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].members.len(), 2);
        assert!(result[0].members.contains(&"A".to_string()));
        assert!(result[0].members.contains(&"B".to_string()));
        assert_eq!(result[1].members.len(), 2);
        assert!(result[1].members.contains(&"C".to_string()));
        assert!(result[1].members.contains(&"D".to_string()));
    }

    #[test]
    fn test_reorganize_batch_groups_splits_large_group() {
        // Test that a 4-person group is split into 2+2
        let mut group1 = Group::new();
        group1.members = vec!["A".to_string(), "B".to_string(), "C".to_string(), "D".to_string()];

        let groups = vec![group1];
        let result = reorganize_batch_groups(groups);

        // 4-person group should be split into 2+2
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].members.len(), 2);
        assert_eq!(result[1].members.len(), 2);
    }

    #[test]
    fn test_reorganize_batch_groups_splits_five_person_group() {
        // Test that a 5-person group is split into 3+2
        let mut group1 = Group::new();
        group1.members = vec!["A".to_string(), "B".to_string(), "C".to_string(), "D".to_string(), "E".to_string()];

        let groups = vec![group1];
        let result = reorganize_batch_groups(groups);

        // 5-person group should be split into 3+2
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].members.len(), 3);
        assert_eq!(result[1].members.len(), 2);
    }

    #[test]
    fn test_reorganize_batch_groups_splits_seven_person_group() {
        // Test that a 7-person group is split into 3+2+2
        let mut group1 = Group::new();
        group1.members = vec![
            "A".to_string(), "B".to_string(), "C".to_string(), "D".to_string(),
            "E".to_string(), "F".to_string(), "G".to_string()
        ];

        let groups = vec![group1];
        let result = reorganize_batch_groups(groups);

        // 7-person group should be split into 3+2+2
        assert_eq!(result.len(), 3);
        let sizes: Vec<usize> = result.iter().map(|g| g.members.len()).collect();
        assert_eq!(sizes, vec![3, 2, 2]);
    }

    #[test]
    fn test_reorganize_batch_groups_no_single_person_groups() {
        // Verify that no single-person groups are ever created
        for total in 2..=20 {
            let mut group = Group::new();
            for i in 0..total {
                group.members.push(format!("S{:03}", i));
            }
            let groups = vec![group];
            let result = reorganize_batch_groups(groups);
            
            for g in &result {
                assert!(g.members.len() >= 2, "Group with {} members found for total {}", g.members.len(), total);
                assert!(g.members.len() <= 3, "Group with {} members found for total {}", g.members.len(), total);
            }
            
            let total_after: usize = result.iter().map(|g| g.members.len()).sum();
            assert_eq!(total_after, total, "Total members should be preserved");
        }
    }
}
