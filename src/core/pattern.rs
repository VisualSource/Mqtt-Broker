const SINGLE_LEVEL_WILD: &str = "+";
const MULTI_LEVEL_WILD: &str = "#";
const VARIABLE_IDENT: &str = "$";
const SEPERATOR: char = '/';

pub struct Pattern<'a> {
    pattern: Vec<&'a str>,
}

impl<'a> Pattern<'a> {
    pub fn new(topic: &'a str) -> Self {
        Self {
            pattern: Self::split_inclusive(topic),
        }
    }

    // https://stackoverflow.com/questions/32257273/split-a-string-keeping-the-separators
    fn split_inclusive(value: &'a str) -> Vec<&'a str> {
        let mut result = Vec::new();
        let mut last: usize = 0;
        for (index, matched) in value.match_indices(SEPERATOR) {
            if last != index {
                result.push(&value[last..index]);
            }
            result.push(matched);
            last = index + matched.len();
        }
        if last < value.len() {
            result.push(&value[last..]);
        }
        result
    }

    pub fn matches(&self, topic: &'a str) -> bool {
        let pattern = Self::split_inclusive(topic);

        let mut pattern_iter = pattern.iter();

        let mut target_iter = self.pattern.iter();

        // Pattern:  test / #
        // TARGET:   test / hello / test

        // Pattern: test/
        // TARGET:  test/hello
        let mut idx = 0;
        loop {
            idx += 1;
            let t = target_iter.next(); // world
            let p = pattern_iter.next(); // test

            if t.is_none() && p.is_none() {
                return true;
            }

            if t.is_none() || p.is_none() {
                return false;
            }

            let part = p.expect("Failed to unwrap el");
            let target = t.expect("Failed to unwrap");

            if part == &MULTI_LEVEL_WILD && !(target.starts_with(VARIABLE_IDENT) && idx == 1) {
                return true;
            }

            if part == &SINGLE_LEVEL_WILD && !(target.starts_with(VARIABLE_IDENT) && idx == 1) {
                continue;
            }

            if part == target {
                continue;
            }

            return false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Pattern;

    #[test]
    fn test_single_topic() {
        let topic = "topic/test".to_string();

        let subscribtion = "topic/test".to_string();

        let pattern = Pattern::new(&topic);

        assert!(pattern.matches(&subscribtion))
    }

    #[test]
    fn test_single_level_wild_topic() {
        let topic = "topic/test/test".to_string();

        let subscribtion = "topic/+/test".to_string();

        let pattern = Pattern::new(&topic);

        assert!(pattern.matches(&subscribtion))
    }

    #[test]
    fn test_multi_level_wild_topic() {
        let topic = "topic/test/test".to_string();

        let subscribtion = "topic/#".to_string();

        let pattern = Pattern::new(&topic);

        assert!(pattern.matches(&subscribtion))
    }

    #[test]
    fn test_non_match_topic() {
        let topic = "topic/test/test".to_string();

        let subscribtion = "topic/".to_string();

        let pattern = Pattern::new(&topic);

        assert!(!pattern.matches(&subscribtion))
    }

    #[test]
    fn test_ident_mutli_wild_topic() {
        let topic = "$SYS".to_string();

        let subscribtion = "#".to_string();

        let pattern = Pattern::new(&topic);

        assert!(!pattern.matches(&subscribtion))
    }

    #[test]
    fn test_ident_single_wild_topic() {
        let topic = "$SYS".to_string();

        let subscribtion = "+".to_string();

        let pattern = Pattern::new(&topic);

        assert!(!pattern.matches(&subscribtion))
    }

    #[test]
    fn test_match_doller_not_at_start() {
        let topic = "test/$SYS".to_string();

        let subscribtion = "test/+".to_string();

        let pattern = Pattern::new(&topic);

        assert!(pattern.matches(&subscribtion))
    }
}
