use crate::config;

fn extract_number_sequences(value: &str, amount: usize) -> &str {
    let mut start: Option<usize> = None;
    let mut iterator = value.chars().enumerate();
    for (i, c) in iterator.by_ref() {
        if c.is_ascii_digit() {
            start = Some(i);
            break;
        } else if c != '-' {
            return "";
        }
    }
    let Some(start) = start else {
        return ""; // There's no number sequence in string.
    };
    let mut end = start + 1;

    let mut remaining = amount;
    let mut on_separator = false;
    for (i, c) in iterator.by_ref() {
        if c.is_ascii_digit() {
            on_separator = false;
            end = i + 1;
        } else if c == '-' {
            if on_separator {
                return ""; // Repeating separators.
            }
            remaining -= 1;
            if remaining == 0 {
                break;
            }
            on_separator = true;
        } else if c.is_alphabetic() {
            break;
        }
    }
    if remaining > 1 {
        return ""; // Couldn't find enough matches.
    }
    &value[start..end]
}

pub fn find_issue_reference(config: &config::Config, branch: &str) -> Option<String> {
    if !prefix_matches(branch, &config.branch_prefixes) {
        return None;
    }
    let mut index: Option<usize> = Some(0);
    while let Some(i) = index {
        for code in &config.codes {
            if branch[i..].starts_with(code) {
                let rest = &branch[i + code.len()..];
                let id = extract_number_sequences(rest, 1);
                if id.is_empty() {
                    return None;
                }
                return Some([code, id].join("-"));
            };
        }
        if branch[i..].starts_with("CVE") {
            let rest = &branch[i + 3..];
            let id = extract_number_sequences(rest, 2);
            if id.is_empty() {
                return None;
            }
            return Some(["CVE", id].join("-"));
        }
        index = branch[i..].find('/').map(|x| x + 1);
    }
    None
}

fn prefix_matches(branch: &str, prefixes: &Vec<String>) -> bool {
    if prefixes.is_empty() {
        return true;
    }
    for prefix in prefixes {
        if branch.starts_with(&format!("{}/", prefix)) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use config::Config;
    use test_case::test_case;

    #[test_case("--666", 1, "666" ; "number with double prefix")]
    #[test_case("-1337", 1, "1337" ; "number with prefix")]
    #[test_case("-1999-2025", 1, "1999" ; "first number with prefix")]
    #[test_case("-72-x", 1, "72" ; "number with prefix and suffix")]
    #[test_case("1234", 1, "1234" ; "number")]
    #[test_case("2020-2025", 1, "2020" ; "first number")]
    #[test_case("2024-2025", 2, "2024-2025" ; "two numbers")]
    #[test_case("2024-2025", 3, "" ; "overflowed numbers")]
    #[test_case("2024-2032-years", 2, "2024-2032" ; "two numbers with suffix")]
    #[test_case("333--444", 2, "" ; "two numbers with double-separator")]
    #[test_case("99-suffix", 1, "99" ; "number with suffix")]
    #[test_case("blink-182", 1, "" ; "number after text")]
    #[test_case("no-numbers", 1, "" ; "text only")]
    fn extract_number_sequences(input: &str, amount: usize, expected: &str) {
        let output = super::extract_number_sequences(&input, amount);
        assert_eq!(output, expected);
    }

    #[test_case("CVE-2024", None ; "not cve")]
    #[test_case("CVE-2024-53908", Some("CVE-2024-53908") ; "cve")]
    #[test_case("CVE-2024-53908-SQL-injection", Some("CVE-2024-53908") ; "cve with suffix")]
    #[test_case("CVE-202453908", None ; "cve without separator")]
    #[test_case("feature/GG-666", None ; "incorrect prefixed code with dash")]
    #[test_case("feature/GG1234", None ; "incorrect prefixed code without dash")]
    #[test_case("feature/GH-666", Some("GH-666") ; "prefixed with dash")]
    #[test_case("feature/GH-666-my-issue", Some("GH-666") ; "prefixed with dash and suffix")]
    #[test_case("feature/GH1234", Some("GH-1234") ; "prefixed without dash")]
    #[test_case("feature/GH1234-my-issue", Some("GH-1234") ; "prefixed without dash with suffix")]
    #[test_case("GG-7", None ; "incorrect code with dash")]
    #[test_case("GG15", None ; "incorrect code without dash")]
    #[test_case("GH-7", Some("GH-7") ; "with dash")]
    #[test_case("GH-7-fix", Some("GH-7") ; "with dash and suffix")]
    #[test_case("GH15", Some("GH-15") ; "without dash")]
    #[test_case("GH15-fix", Some("GH-15") ; "without dash with suffix")]
    #[test_case("master", None ; "master")]
    #[test_case("security/CVE-2024-53908", Some("CVE-2024-53908") ; "prefixed cve")]
    #[test_case("security/CVE-2024-53908-SQL-injection", Some("CVE-2024-53908") ; "prefixed cve with suffix")]
    fn find_issue_reference_single(branch: &str, expected: Option<&str>) {
        let config = Config {
            codes: vec!["GH".to_string()],
            branch_prefixes: vec![],
        };
        let reference = find_issue_reference(&config, branch);
        assert_eq!(reference, expected.map(|s| s.to_string()));
    }

    #[test_case("feature/GG-666", Some("GG-666") ; "prefixed with dash")]
    #[test_case("feature/GG1234", Some("GG-1234") ; "prefixed without dash")]
    #[test_case("feature/GH-666", Some("GH-666") ; "another prefixed with dash")]
    #[test_case("feature/GH-666-my-issue", Some("GH-666") ; "prefixed with dash and suffix")]
    #[test_case("feature/GH1234", Some("GH-1234") ; "another prefixed without dash")]
    #[test_case("feature/GH1234-my-issue", Some("GH-1234") ; "prefixed without dash and suffix")]
    #[test_case("GG-7", Some("GG-7") ; "with dash")]
    #[test_case("GG15", Some("GG-15") ; "without dash")]
    #[test_case("GGG-7", None ; "incorrect with dash")]
    #[test_case("GGG15", None ; "incorrect without dash")]
    #[test_case("GH-7", Some("GH-7") ; "another with dash")]
    #[test_case("GH-7-fix", Some("GH-7") ; "with dash and suffix")]
    #[test_case("GH15", Some("GH-15") ; "another without dash")]
    #[test_case("GH15-fix", Some("GH-15") ; "without dash with suffix")]
    #[test_case("master", None ; "master")]
    fn find_issue_reference_multi(branch: &str, expected: Option<&str>) {
        let config = Config {
            codes: vec!["GH".to_string(), "GG".to_string()],
            branch_prefixes: vec![],
        };
        let reference = find_issue_reference(&config, branch);
        assert_eq!(reference, expected.map(|s| s.to_string()));
    }

    #[test_case("feature/TEAM1-2345", Some("TEAM1-2345") ; "prefixed with dash")]
    #[test_case("feature/TEAM12345", Some("TEAM1-2345") ; "prefixed without dash")]
    #[test_case("feature/TEAM2-3456", None ; "prefixed incorrect with dash")]
    #[test_case("feature/TEAM23456", None ; "prefixed incorrect without dash")]
    #[test_case("TEAM1-2345", Some("TEAM1-2345") ; "with dash")]
    #[test_case("TEAM12345", Some("TEAM1-2345") ; "without dash")]
    #[test_case("TEAM2-3456", None ; "incorrect with dash")]
    #[test_case("TEAM23456", None ; "incorrect without dash")]
    fn find_issue_reference_single_numerical_end(branch: &str, expected: Option<&str>) {
        let config = Config {
            codes: vec!["TEAM1".to_string()],
            branch_prefixes: vec![],
        };
        let reference = find_issue_reference(&config, branch);
        assert_eq!(reference, expected.map(|s| s.to_string()));
    }

    #[test_case("feature/TEAM1-2345", Some("TEAM1-2345") ; "prefixed with dash")]
    #[test_case("feature/TEAM12345", Some("TEAM1-2345") ; "prefixed without dash")]
    #[test_case("feature/TEAM2-3456", Some("TEAM2-3456") ; "another prefixed with dash")]
    #[test_case("feature/TEAM23456", Some("TEAM2-3456") ; "another prefixed without dash")]
    #[test_case("feature/TEAM3-4567", None ; "incorrect prefixed with dash")]
    #[test_case("feature/TEAM34567", None ; "incorrect prefixed without dash")]
    #[test_case("TEAM1-2345", Some("TEAM1-2345") ; "with dash")]
    #[test_case("TEAM12345", Some("TEAM1-2345") ; "without dash")]
    #[test_case("TEAM2-3456", Some("TEAM2-3456") ; "another with dash")]
    #[test_case("TEAM23456", Some("TEAM2-3456") ; "another without dash")]
    #[test_case("TEAM3-3457", None ; "incorrect with dash")]
    #[test_case("TEAM34567", None ; "incorrect without dash")]
    fn find_issue_reference_multi_numerical_end(branch: &str, expected: Option<&str>) {
        let config = Config {
            codes: vec!["TEAM1".to_string(), "TEAM2".to_string()],
            branch_prefixes: vec![],
        };
        let reference = find_issue_reference(&config, branch);
        assert_eq!(reference, expected.map(|s| s.to_string()));
    }

    #[test_case("bug/GIT-1234", None ; "incorrect prefix with dash")]
    #[test_case("bug/GIT-1234-feature", None ; "incorrect prefix with dash and suffix")]
    #[test_case("bug/GIT1234", None ; "incorrect prefix without dash")]
    #[test_case("bug/GIT1234-feature", None ; "incorrect prefix with suffix without dash")]
    #[test_case("CVE-2024-53908", None ; "unprefixed cve")]
    #[test_case("feature/GIT-1234", Some("GIT-1234") ; "correct prefix with dash")]
    #[test_case("feature/GIT-1234-feature", Some("GIT-1234") ; "correct prefix with dash and suffix")]
    #[test_case("feature/GIT1234", Some("GIT-1234") ; "correct prefix without dash")]
    #[test_case("feature/GIT1234-feature", Some("GIT-1234") ; "correct prefix with suffix without dash")]
    #[test_case("GIT-1234", None ; "unprefixed with dash")]
    #[test_case("GIT-1234-feature", None ; "unprefix with dash and suffix")]
    #[test_case("GIT1234", None ; "unprefixed without dash")]
    #[test_case("GIT1234-feature", None ; "unprefix with suffix without dash")]
    #[test_case("security/CVE-2024-53908", Some("CVE-2024-53908") ; "correct prefix cve")]
    #[test_case("squash/CVE-2024-53908", None ; "incorrect prefix cve")]
    fn find_issue_reference_prefixing(branch: &str, expected: Option<&str>) {
        let config = Config {
            codes: vec!["GIT".to_string()],
            branch_prefixes: vec!["feature".to_string(), "security".to_string()],
        };
        let reference = find_issue_reference(&config, branch);
        assert_eq!(reference, expected.map(|s| s.to_string()));
    }
}
