use schemadiff::backend::{self, Cmd, Reply, Which};
use schemadiff::compare::{CompareResult, ObjectDiff};
use schemadiff::{export, tree};
use slint::{ModelRc, SharedString, VecModel};
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;
use std::sync::mpsc;

slint::include_modules!();

struct Store {
    result: Option<CompareResult>,
    expanded: BTreeSet<(usize, Option<usize>)>,
    selected: Option<(usize, usize)>,
}

fn push_tree(
    model: &VecModel<TreeRow>,
    res: &CompareResult,
    expanded: &BTreeSet<(usize, Option<usize>)>,
    sel: Option<(usize, usize)>,
) {
    let entries = tree::flatten(res, expanded, sel);
    model.set_vec(
        entries
            .iter()
            .map(|e| TreeRow {
                kind: match e.row_kind {
                    tree::TreeRowKind::Group => "group",
                    tree::TreeRowKind::Object => "object",
                    tree::TreeRowKind::Detail => "detail",
                }
                .into(),
                depth: e.depth as i32,
                expanded: e.expanded,
                selected: e.selected,
                label: e.label.clone().into(),
                count: e.count,
            })
            .collect::<Vec<_>>(),
    );
}

fn push_diff(model: &VecModel<DiffRow>, o: &ObjectDiff) {
    model.set_vec(
        o.lines
            .iter()
            .map(|l| DiffRow {
                kind: match l.kind {
                    schemadiff::diff::LineKind::Added => "added",
                    schemadiff::diff::LineKind::Removed => "removed",
                    schemadiff::diff::LineKind::Modified => "modified",
                    schemadiff::diff::LineKind::Unchanged => "unchanged",
                }
                .into(),
                source: l.source.clone().into(),
                target: l.target.clone().into(),
            })
            .collect::<Vec<_>>(),
    );
}

/// Set the status-bar text; `is_error` drives the red color via State.status-error.
fn status(state: &State, text: impl Into<SharedString>, is_error: bool) {
    state.set_status(text.into());
    state.set_status_error(is_error);
}

fn set_clipboard(state: &State, text: String) {
    match arboard::Clipboard::new().and_then(|mut c| c.set_text(text)) {
        Ok(()) => status(state, "Copied to clipboard.", false),
        Err(e) => status(state, format!("Clipboard failed: {e}"), true),
    }
}

fn save_file(state: &State, default_name: &str, filter_name: &str, ext: &str, body: String) {
    let path = rfd::FileDialog::new()
        .set_file_name(default_name)
        .add_filter(filter_name, &[ext])
        .save_file();
    if let Some(p) = path {
        match std::fs::write(&p, body) {
            Ok(()) => status(state, format!("Saved {}", p.display()), false),
            Err(e) => status(state, format!("Save failed: {e}"), true),
        }
    }
}

fn main() -> Result<(), slint::PlatformError> {
    let app = AppWindow::new()?;
    let state = app.global::<State>();

    let (reply_tx, reply_rx) = mpsc::channel::<Reply>();
    let cmd_tx = backend::start(reply_tx);
    let store = Rc::new(RefCell::new(Store {
        result: None,
        expanded: BTreeSet::new(),
        selected: None,
    }));

    let tree_model = Rc::new(VecModel::<TreeRow>::from(vec![]));
    let diff_model = Rc::new(VecModel::<DiffRow>::from(vec![]));
    state.set_tree(ModelRc::from(tree_model.clone()));
    state.set_diff(ModelRc::from(diff_model.clone()));

    {
        let cmd_tx = cmd_tx.clone();
        let weak = app.as_weak();
        state.on_connect(move |is_source, cs| {
            let Some(app) = weak.upgrade() else { return };
            let st = app.global::<State>();
            if is_source {
                st.set_source_state(0);
            } else {
                st.set_target_state(0);
            }
            status(&st, "Connecting…", false);
            if cmd_tx
                .send(Cmd::Test {
                    which: if is_source { Which::Source } else { Which::Target },
                    cs: cs.to_string(),
                })
                .is_err()
            {
                status(&st, "Backend unavailable.", true);
                st.set_busy(false);
            }
        });
    }

    {
        let cmd_tx = cmd_tx.clone();
        let store = store.clone();
        let tree_model = tree_model.clone();
        let diff_model = diff_model.clone();
        let weak = app.as_weak();
        state.on_compare(move |src_cs, tgt_cs| {
            let Some(app) = weak.upgrade() else { return };
            let st = app.global::<State>();
            st.set_busy(true);
            status(&st, "Comparing…", false);
            tree_model.set_vec(vec![]);
            diff_model.set_vec(vec![]);
            st.set_diff_title("".into());
            {
                let mut s = store.borrow_mut();
                s.result = None;
                s.expanded.clear();
                s.selected = None;
            }
            if cmd_tx
                .send(Cmd::Compare {
                    src_cs: src_cs.to_string(),
                    tgt_cs: tgt_cs.to_string(),
                })
                .is_err()
            {
                status(&st, "Backend unavailable.", true);
                st.set_busy(false);
            }
        });
    }

    {
        let store = store.clone();
        let tree_model = tree_model.clone();
        let diff_model = diff_model.clone();
        let weak = app.as_weak();
        state.on_tree_clicked(move |idx| {
            let Some(app) = weak.upgrade() else { return };
            let mut st = store.borrow_mut();
            let Store {
                result,
                expanded,
                selected,
            } = &mut *st;
            let Some(res) = result.as_ref() else { return };
            let rows = tree::flatten(res, expanded, *selected);
            let Some(row) = rows.get(idx as usize) else { return };
            let (g, o) = (row.group, row.object);
            match row.row_kind {
                tree::TreeRowKind::Group => {
                    let k = (g, None);
                    if !expanded.remove(&k) {
                        expanded.insert(k);
                    }
                }
                tree::TreeRowKind::Object => {
                    let k = (g, o.unwrap());
                    if !expanded.remove(&(k.0, Some(k.1))) {
                        expanded.insert((k.0, Some(k.1)));
                    }
                    *selected = Some(k);
                    let obj = &res.groups[k.0].objects[k.1];
                    push_diff(&diff_model, obj);
                    app.global::<State>()
                        .set_diff_title(format!("{}.{}", obj.schema, obj.name).into());
                }
                tree::TreeRowKind::Detail => {
                    if let Some(oi) = o {
                        *selected = Some((g, oi));
                    }
                }
            }
            push_tree(&tree_model, res, expanded, *selected);
        });
    }

    {
        let store = store.clone();
        let weak = app.as_weak();
        state.on_copy_object(move || {
            let Some(app) = weak.upgrade() else { return };
            let text = {
                let s = store.borrow();
                match (&s.result, s.selected) {
                    (Some(res), Some((g, o))) => {
                        Some(export::unified_text(&res.groups[g].objects[o]))
                    }
                    _ => None,
                }
            };
            match text {
                Some(t) => set_clipboard(&app.global::<State>(), t),
                None => status(&app.global::<State>(), "Nothing selected.", false),
            }
        });
    }

    {
        let store = store.clone();
        let weak = app.as_weak();
        state.on_copy_all(move || {
            let Some(app) = weak.upgrade() else { return };
            let text = store.borrow().result.as_ref().map(export::unified_text_all);
            if let Some(t) = text {
                set_clipboard(&app.global::<State>(), t);
            }
        });
    }

    {
        let store = store.clone();
        let weak = app.as_weak();
        state.on_export_object(move || {
            let Some(app) = weak.upgrade() else { return };
            let st = app.global::<State>();
            let picked = {
                let s = store.borrow();
                match (&s.result, s.selected) {
                    (Some(res), Some((g, o))) => {
                        let obj = &res.groups[g].objects[o];
                        Some((
                            format!("{}.{}.sql", obj.schema, obj.name),
                            export::sql_for_object(obj, res.direction),
                        ))
                    }
                    _ => None,
                }
            };
            match picked {
                Some((name, body)) => save_file(&st, &name, "SQL", "sql", body),
                None => status(&st, "Nothing selected.", false),
            }
        });
    }

    {
        let store = store.clone();
        let weak = app.as_weak();
        state.on_export_all(move |fmt: SharedString| {
            let Some(app) = weak.upgrade() else { return };
            let st = app.global::<State>();
            let picked = {
                let s = store.borrow();
                s.result.as_ref().map(|res| match fmt.as_str() {
                    "json" => export::to_json(res)
                        .map(|b| ("schema-diff.json", "JSON", "json", b)),
                    "csv" => export::to_csv(res)
                        .map(|b| ("schema-diff.csv", "CSV", "csv", b)),
                    _ => Ok(("schema-diff.sql", "SQL", "sql", export::sql_for_all(res))),
                })
            };
            match picked {
                Some(Ok((name, fname, ext, body))) => save_file(&st, name, fname, ext, body),
                Some(Err(e)) => status(&st, format!("Export failed: {e}"), true),
                None => {}
            }
        });
    }

    let timer = slint::Timer::default();
    {
        let store = store.clone();
        let weak = app.as_weak();
        let tree_model = tree_model.clone();
        timer.start(
            slint::TimerMode::Repeated,
            std::time::Duration::from_millis(120),
            move || {
                let Some(app) = weak.upgrade() else { return };
                let st = app.global::<State>();
                while let Ok(r) = reply_rx.try_recv() {
                    match r {
                        Reply::Connected {
                            which_is_source,
                            ok,
                            msg,
                        } => {
                            if which_is_source {
                                st.set_source_state(if ok { 1 } else { 2 });
                            } else {
                                st.set_target_state(if ok { 1 } else { 2 });
                            }
                            if ok {
                                status(&st, "Connected.", false);
                            } else {
                                status(&st, format!("Connection failed: {msg}"), true);
                            }
                        }
                        Reply::Result(res) => {
                            let mut s = store.borrow_mut();
                            s.expanded.clear();
                            s.selected = None;
                            let mut text = format!(
                                "{} objects compared, {} differences found.",
                                res.compared, res.different
                            );
                            if !res.warnings.is_empty() {
                                text.push_str(&format!(
                                    " + {} fetch warning(s): {}",
                                    res.warnings.len(),
                                    res.warnings[0]
                                ));
                            }
                            status(&st, text, false);
                            push_tree(&tree_model, &res, &s.expanded, None);
                            s.result = Some(*res);
                            st.set_busy(false);
                        }
                        Reply::Failed(m) => {
                            status(&st, format!("Compare failed: {m}"), true);
                            st.set_busy(false);
                        }
                    }
                }
            },
        );
    }

    app.run()
}
