use rand::seq::SliceRandom;
use std::io::{self, BufRead};

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

fn read_student_ids() -> Vec<Group> {
    let stdin = io::stdin();
    let mut groups = Vec::new();
    let mut current_group = Group::new();

    println!("学籍番号を入力してください (3人ごとにグループになります。EOFで終了: Ctrl+D (Unix/Mac) または Ctrl+Z (Windows)):");

    for line in stdin.lock().lines() {
        match line {
            Ok(student_id) => {
                let student_id = student_id.trim().to_string();
                if !student_id.is_empty() {
                    current_group.add_member(student_id);

                    if current_group.is_full() {
                        groups.push(current_group);
                        current_group = Group::new();
                    }
                }
            }
            Err(_) => break,
        }
    }

    // Save the current group if it has any members (requirement 2)
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
    let groups = read_student_ids();
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
}
