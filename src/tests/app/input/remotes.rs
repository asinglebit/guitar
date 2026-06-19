use crate::{app::input::remotes::REMOTE_ACTIONS, helpers::localisation::menu};

#[test]
fn remote_action_labels_keep_visible_order() {
    let labels: Vec<&str> = REMOTE_ACTIONS.iter().map(|action| action.label()).collect();

    assert_eq!(labels, vec![menu::FETCH(), menu::SET_AS_DEFAULT(), menu::RENAME_REMOTE(), menu::EDIT_FETCH_URL(), menu::EDIT_PUSH_URL(), menu::DELETE_REMOTE(),]);
}
