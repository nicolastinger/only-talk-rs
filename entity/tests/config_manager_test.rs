use entity::config_manager::{get_config, remove_config, set_config};

#[test]
fn test_set_and_get_config() {
    let test_prefix = "test_set_and_get_";

    set_config(format!("{}database.url", test_prefix), "postgres://localhost/test".to_string());
    set_config(format!("{}server.port", test_prefix), "8080".to_string());

    let db_url = get_config(&format!("{}database.url", test_prefix));
    assert_eq!(db_url, Some("postgres://localhost/test".to_string()));

    let port = get_config(&format!("{}server.port", test_prefix));
    assert_eq!(port, Some("8080".to_string()));

    let not_exist = get_config(&format!("{}not.exist", test_prefix));
    assert_eq!(not_exist, None);

    remove_config(&format!("{}database.url", test_prefix));
    remove_config(&format!("{}server.port", test_prefix));
}

#[test]
fn test_update_config() {
    let test_prefix = "test_update_";

    set_config(format!("{}key1", test_prefix), "value1".to_string());
    assert_eq!(get_config(&format!("{}key1", test_prefix)), Some("value1".to_string()));

    set_config(format!("{}key1", test_prefix), "value2".to_string());
    assert_eq!(get_config(&format!("{}key1", test_prefix)), Some("value2".to_string()));

    remove_config(&format!("{}key1", test_prefix));
}

#[test]
fn test_remove_config() {
    let test_prefix = "test_remove_";

    set_config(format!("{}to_remove", test_prefix), "value".to_string());
    let result = get_config(&format!("{}to_remove", test_prefix));
    assert_eq!(result, Some("value".to_string()));

    let removed = remove_config(&format!("{}to_remove", test_prefix));
    assert_eq!(removed, Some("value".to_string()));

    let result = get_config(&format!("{}to_remove", test_prefix));
    assert_eq!(result, None);

    let removed_again = remove_config(&format!("{}to_remove", test_prefix));
    assert_eq!(removed_again, None);
}

#[test]
fn test_clear_config() {
    let test_prefix = "test_clear_";

    set_config(format!("{}key1", test_prefix), "value1".to_string());
    set_config(format!("{}key2", test_prefix), "value2".to_string());
    set_config(format!("{}key3", test_prefix), "value3".to_string());

    assert_eq!(get_config(&format!("{}key1", test_prefix)), Some("value1".to_string()));
    assert_eq!(get_config(&format!("{}key2", test_prefix)), Some("value2".to_string()));
    assert_eq!(get_config(&format!("{}key3", test_prefix)), Some("value3".to_string()));

    remove_config(&format!("{}key1", test_prefix));
    remove_config(&format!("{}key2", test_prefix));
    remove_config(&format!("{}key3", test_prefix));

    assert_eq!(get_config(&format!("{}key1", test_prefix)), None);
    assert_eq!(get_config(&format!("{}key2", test_prefix)), None);
    assert_eq!(get_config(&format!("{}key3", test_prefix)), None);
}

#[test]
fn test_empty_key_value() {
    let test_prefix = "test_empty_";

    set_config(format!("{}{}", test_prefix, ""), "value".to_string());
    assert_eq!(get_config(&format!("{}{}", test_prefix, "")), Some("value".to_string()));

    set_config(format!("{}key", test_prefix), "".to_string());
    assert_eq!(get_config(&format!("{}key", test_prefix)), Some("".to_string()));

    set_config(format!("{}{}", test_prefix, ""), "".to_string());
    assert_eq!(get_config(&format!("{}{}", test_prefix, "")), Some("".to_string()));

    remove_config(&format!("{}{}", test_prefix, ""));
    remove_config(&format!("{}key", test_prefix));
}

#[test]
fn test_special_characters() {
    let test_prefix = "test_special_";

    let special_key = format!("{}key.with.dots_and-underscores", test_prefix);
    let special_value = "value with spaces and 特殊字符";

    set_config(special_key.clone(), special_value.to_string());
    assert_eq!(get_config(&special_key), Some(special_value.to_string()));

    remove_config(&special_key);
}

#[test]
fn test_concurrent_operations() {
    let test_prefix = "test_concurrent_";

    use std::thread;

    let handles: Vec<_> = (0..10)
        .map(|i| {
            let prefix = test_prefix.to_string();
            thread::spawn(move || {
                let key = format!("{}key_{}", prefix, i);
                let value = format!("value_{}", i);
                set_config(key.clone(), value.clone());
                let result = get_config(&key);
                result
            })
        })
        .collect();

    let results: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().expect("Thread panicked"))
        .collect();

    for (i, result) in results.iter().enumerate() {
        let expected = format!("value_{}", i);
        assert_eq!(result, &Some(expected));
    }

    for i in 0..10 {
        remove_config(&format!("{}key_{}", test_prefix, i));
    }
}

#[test]
fn test_overwrite_existing_key() {
    let test_prefix = "test_overwrite_";

    set_config(format!("{}key", test_prefix), "original".to_string());
    assert_eq!(get_config(&format!("{}key", test_prefix)), Some("original".to_string()));

    set_config(format!("{}key", test_prefix), "new_value".to_string());
    assert_eq!(get_config(&format!("{}key", test_prefix)), Some("new_value".to_string()));

    assert_eq!(remove_config(&format!("{}key", test_prefix)), Some("new_value".to_string()));
    assert_eq!(get_config(&format!("{}key", test_prefix)), None);
}

#[test]
fn test_multiple_operations_sequence() {
    let test_prefix = "test_sequence_";

    set_config(format!("{}a", test_prefix), "1".to_string());
    set_config(format!("{}b", test_prefix), "2".to_string());
    set_config(format!("{}c", test_prefix), "3".to_string());

    assert_eq!(get_config(&format!("{}a", test_prefix)), Some("1".to_string()));
    assert_eq!(get_config(&format!("{}b", test_prefix)), Some("2".to_string()));
    assert_eq!(get_config(&format!("{}c", test_prefix)), Some("3".to_string()));

    remove_config(&format!("{}b", test_prefix));
    assert_eq!(get_config(&format!("{}b", test_prefix)), None);
    assert_eq!(get_config(&format!("{}a", test_prefix)), Some("1".to_string()));
    assert_eq!(get_config(&format!("{}c", test_prefix)), Some("3".to_string()));

    set_config(format!("{}d", test_prefix), "4".to_string());
    assert_eq!(get_config(&format!("{}d", test_prefix)), Some("4".to_string()));

    remove_config(&format!("{}a", test_prefix));
    remove_config(&format!("{}c", test_prefix));
    remove_config(&format!("{}d", test_prefix));
}
