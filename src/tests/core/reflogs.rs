use super::*;
use git2::Oid;
use ratatui::style::Color;

#[test]
fn duplicate_events_remain_list_rows_but_latest_is_newest() {
    let oid = Oid::from_str("1111111111111111111111111111111111111111").unwrap();
    let alias = 1;

    let newest = HeadReflogAliasEntry { selector: "HEAD@{0}".to_string(), old_oid: Oid::zero(), new_oid: oid, new_alias: alias, message: "newest".to_string(), time: gix::date::Time::new(2, 0) };
    let older = HeadReflogAliasEntry { selector: "HEAD@{1}".to_string(), old_oid: Oid::zero(), new_oid: oid, new_alias: alias, message: "older".to_string(), time: gix::date::Time::new(1, 0) };
    let reflogs = HeadReflogs { entries: vec![newest.clone(), older], latest_by_alias: [(alias, newest)].into_iter().collect(), colors: [(alias, Color::Green)].into_iter().collect() };

    assert_eq!(reflogs.entries.len(), 2);
    assert_eq!(reflogs.latest_for_alias(alias).unwrap().message, "newest");
    assert_eq!(reflogs.get_color(alias), Some(Color::Green));
}
