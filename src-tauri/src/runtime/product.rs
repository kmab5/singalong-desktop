use super::{lookup::Coordinator, search::Search, tracking::Engine};
use singalong_core::model::*;
pub(super) fn execute(
    engine: &mut Engine,
    lookup: &mut Option<Coordinator>,
    search: &mut Option<Search>,
    command: ProductCommand,
    now: u64,
    overlay: bool,
) -> Result<ProductReply, String> {
    if overlay {
        match &command {
            ProductCommand::Reader { action } => {
                let id = match action {
                    ReaderAction::Select { version_id }
                    | ReaderAction::Adjust { version_id, .. }
                    | ReaderAction::Align { version_id, .. } => version_id,
                };
                if !lookup
                    .as_ref()
                    .is_some_and(|l| l.state.versions.iter().any(|v| &v.id == id))
                {
                    return Err("The live lyric version changed.".into());
                }
            }
            ProductCommand::Overlay { .. } | ProductCommand::ClickThrough { .. } => {}
            _ => return Err("This window cannot perform that action.".into()),
        }
    }
    match command {
        ProductCommand::CurrentDetail => {
            let recording = engine
                .selected()
                .and_then(|s| s.recording)
                .ok_or("No current recording.")?;
            let saved = engine
                .db
                .resolve(recording, None)
                .map_err(|e| e.to_string())?;
            return engine
                .db
                .detail(saved.id)
                .map(|detail| ProductReply::Detail { detail })
                .map_err(|e| e.to_string());
        }
        ProductCommand::Library { query } => {
            let (items, more) = engine.db.library(query).map_err(|e| e.to_string())?;
            return Ok(ProductReply::Library { items, more });
        }
        ProductCommand::Detail { id } => {
            return engine
                .db
                .detail(id)
                .map(|detail| ProductReply::Detail { detail })
                .map_err(|e| e.to_string());
        }
        ProductCommand::Favorite { id, value } => {
            engine.db.favorite(id, value).map_err(|e| e.to_string())?
        }
        ProductCommand::Reader { action } => {
            let session = engine.selected();
            lookup
                .as_mut()
                .ok_or("Reader unavailable.")?
                .reader_action(action, session.as_ref(), now)?;
        }
        ProductCommand::Appearance { value } => {
            if !(14..=64).contains(&value.main.text_size)
                || !(14..=64).contains(&value.floating.text_size)
                || !value.opacity.is_finite()
                || !(0.2..=1.0).contains(&value.opacity)
            {
                return Err("Appearance values are outside the supported range.".into());
            }
            let mut settings = engine.settings.clone();
            settings.appearance = value;
            engine
                .db
                .save_settings(settings.clone())
                .map_err(|e| e.to_string())?;
            engine.snapshot.appearance = settings.appearance.clone();
            engine.settings = settings;
        }
        ProductCommand::ResetAppearance => {
            let mut settings = engine.settings.clone();
            settings.appearance = Appearance::default();
            engine
                .db
                .save_settings(settings.clone())
                .map_err(|e| e.to_string())?;
            engine.snapshot.appearance = settings.appearance.clone();
            engine.settings = settings;
        }
        ProductCommand::Search { query } => {
            return search
                .as_mut()
                .ok_or("Search unavailable.")?
                .start(query, engine.settings.offline);
        }
        ProductCommand::SearchPoll { token } => {
            return search
                .as_mut()
                .ok_or("Search unavailable.")?
                .poll(&token, engine.settings.offline);
        }
        ProductCommand::SearchCancel { context } => {
            if let Some(search) = search {
                search.cancel_context(&context);
            }
        }
        ProductCommand::SaveSearch { token, version_id } => {
            let result = search
                .as_mut()
                .ok_or("Search unavailable.")?
                .save(&token, &version_id)?;
            if let Some(lookup) = lookup {
                lookup.refresh_saved();
            }
            return Ok(result);
        }
        ProductCommand::ClearDownloads => {
            if let Some(search) = search {
                search.cancel();
            }
            if let Some(lookup) = lookup {
                lookup.downloads_cleared();
            }
            engine.db.clear_downloads().map_err(|e| e.to_string())?;
            if let Some(lookup) = lookup {
                lookup.reload_saved_after_clear();
            }
        }
        ProductCommand::ClearHistory => engine.clear_history()?,
        ProductCommand::Overlay { mode } => {
            let mut settings = engine.settings.clone();
            settings.overlay_mode = mode;
            engine
                .db
                .save_settings(settings.clone())
                .map_err(|e| e.to_string())?;
            engine.settings = settings;
            engine.snapshot.overlay_mode = mode;
        }
        ProductCommand::ClickThrough { enabled } => engine.snapshot.click_through = enabled,
        ProductCommand::BackupPreview { .. }
        | ProductCommand::BackupApply { .. }
        | ProductCommand::BackupExport { .. }
        | ProductCommand::BackupPoll
        | ProductCommand::BackupCancel
        | ProductCommand::DiagnosticsPreview
        | ProductCommand::DiagnosticsExport
        | ProductCommand::HideOverlay
        | ProductCommand::Dock { .. }
        | ProductCommand::ResetOverlay
        | ProductCommand::OpenReleases => {
            return Err("This action requires the native shell.".into());
        }
    }
    Ok(ProductReply::Done)
}
