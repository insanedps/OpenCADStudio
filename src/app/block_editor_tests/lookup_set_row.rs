use super::*;

/// The `Lookup Set` row has to reach the table editor on its own: the command
/// it used to run is the one the action-bar preference refuses.
#[test]
fn lookup_set_row_opens_the_table_editor_without_the_action_bar_refusal() {
    let (mut app, reference) = fixture();
    begin(&mut app, reference);
    let _ = app.dispatch_command("BACTIONBARMODE 1");
    let i = app.active_tab;
    let _ = app.dispatch_command("BPARAMETERSET Lookup Set");
    feed(&mut app, "1,1");
    feed(&mut app, "");
    assert!(
        app.command_line.last_error.is_none(),
        "the row is not refused: {:?}",
        app.command_line.last_error
    );
    assert!(
        app.tabs[i].active_cmd.is_none(),
        "the lookup placement finished, still prompting {:?}",
        app.tabs[i]
            .active_cmd
            .as_ref()
            .map(|command| command.prompt().to_string())
    );
    assert!(
        app.block_lookup.is_some(),
        "the table editor opens for the parameter the row just placed"
    );
    assert_eq!(
        app.active_modal,
        Some(crate::app::ModalKind::BlockLookup)
    );
    assert_eq!(
        crate::scene::block_lookup::parameters(&app.tabs[i].scene.document, app.tabs[i]
            .active_block_edit_session()
            .unwrap()
            .br_handle)
        .len(),
        1,
        "one parameter, authored once"
    );
}
