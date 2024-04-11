/// Parse a topic string into its parts and parse out $share
pub fn tokenise_topic(value: String) -> Result<(Vec<String>, Option<String>), u8> {
    let mut topic = value.split('/').collect::<Vec<&str>>();

    let sharename = if let Some(&"$share") = topic.first() {
        if topic.len() < 2 {
            return Err(1);
        }
        topic.remove(0);
        let share = topic.first().ok_or(1)?.to_string();
        if let Some(first) = topic.first_mut() {
            *first = "";
        }

        Some(share)
    } else {
        None
    };

    Ok((topic.iter().map(|x| x.to_string()).collect(), sharename))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_topic() {
        if let Ok((topic, share)) = tokenise_topic("topic/Hello".to_string()) {
            assert!(share.is_none());
            assert_eq!(topic, vec!["topic".to_string(), "Hello".to_string()]);
        } else {
            panic!("Failed to parse topic")
        }
    }
    #[test]
    fn test_tokenize_topic_with_leading_slash() {
        if let Ok((topic, share)) = tokenise_topic("/topic/Hello".to_string()) {
            assert!(share.is_none());
            assert_eq!(
                topic,
                vec!["".to_string(), "topic".to_string(), "Hello".to_string()]
            );
        } else {
            panic!("Failed to parse topic")
        }
    }
    #[test]
    fn test_tokenize_topic_with_share() {
        if let Ok((topic, share)) = tokenise_topic("$share/GroupA/topic/Hello".to_string()) {
            println!("{:#?} {:#?}", topic, share);
            assert!(share.is_some_and(|x| x == *"GroupA"), "Share is not GroupA");

            assert_eq!(
                topic,
                vec!["".to_string(), "topic".to_string(), "Hello".to_string()]
            );
        } else {
            panic!("Failed to parse topic")
        }
    }
}
