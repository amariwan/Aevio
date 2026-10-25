use super::*;

fn add_current_totals(
    totals: &mut crate::database::repositories::activity::ActivityTotals,
    current: Option<&Current>,
    since_ms: u64,
    now: u64,
) {
    let Some(current) = current else {
        return;
    };
    let active_ms = now.saturating_sub(current.started_at.max(since_ms));
    match current.kind.as_str() {
        KIND_ACTIVITY => {
            totals.tracked_ms += active_ms;
            if !current.reviewed {
                totals.unreviewed += 1;
            }
        }
        KIND_FOCUS => {
            totals.tracked_ms += active_ms;
            totals.focus_ms += active_ms;
        }
        KIND_MEETING => totals.tracked_ms += active_ms,
        KIND_BREAK => totals.break_ms += active_ms,
        _ => {}
    }
}

impl ActivityStore {
    pub fn tick(
        &mut self,
        sample: Option<WindowSample>,
        idle_ms: u64,
        now: u64,
    ) -> Result<bool, String> {
        if sample.as_ref().is_some_and(|s| s.keeps_display_awake) {
            self.watched_at_ms = Some(now);
        }
        // Watching a video gives no input, so while an activity segment is
        // open, the foreground app keeping the display awake counts as the
        // user being there: idle runs from the last input or the last moment
        // they were watching, whichever is later. That also rides out the
        // gaps where a player briefly drops its assertion. It never ends a
        // break, though: only input brings an idle user back.
        let idle_ms = match (&self.current, self.watched_at_ms) {
            (Some(current), Some(watched_at)) if current.kind == KIND_ACTIVITY => {
                idle_ms.min(now.saturating_sub(watched_at))
            }
            _ => idle_ms,
        };
        self.last_idle_ms = idle_ms;

        if !self.capture_enabled {
            self.pending_switch = None;
            // Explicit breaks are a user action, not automatic capture. Keep
            // their segment alive even when the tracker was paused at Start.
            if self.current.as_ref().is_some_and(|current| {
                current.kind == KIND_BREAK && current.label.as_deref() != Some(IDLE_LABEL)
            }) {
                return Ok(false);
            }
            self.manual_tracking = false;
            return self.close_current(now);
        }

        let in_window = self.tracking_hours.is_inside_window(now);
        let idle = idle_ms >= self.idle_threshold_ms;
        let is_active_work = self
            .current
            .as_ref()
            .is_some_and(|cur| cur.kind == KIND_ACTIVITY);

        // Outside tracking hours with no active work session and no manual override:
        // suppress new capture. Close any idle break that was open.
        if !in_window && !self.manual_tracking && !is_active_work {
            self.pending_switch = None;
            if self.current.is_some() {
                self.close_current(now)?;
                return Ok(true);
            }
            return Ok(false);
        }

        // Privacy and exclusion: an excluded app must take effect immediately.
        // It must never be attributed to the previous app, delayed by a grace period,
        // or stored as billable activity.
        if let Some(ref sample) = sample
            && self.is_sample_excluded(sample)
        {
            self.pending_switch = None;
            if is_active_work {
                self.close_current(now)?;
                return Ok(true);
            }
            return Ok(false);
        }

        let current = self.current.as_ref().map(|cur| {
            (
                cur.app.clone(),
                cur.title.clone(),
                cur.kind.clone(),
                cur.label.clone(),
                cur.bundle_id.clone(),
                cur.url.clone(),
            )
        });

        match current {
            None => {
                self.pending_switch = None;
                if !in_window && !self.manual_tracking {
                    return Ok(false);
                }
                if idle {
                    self.open(
                        "Idle",
                        "No activity",
                        KIND_BREAK,
                        Some(IDLE_LABEL),
                        now,
                        None,
                        None,
                        None,
                    )
                } else if let Some(sample) = sample {
                    self.open(
                        &sample.app,
                        &sample.title,
                        KIND_ACTIVITY,
                        None,
                        now,
                        sample.bundle_id.as_deref(),
                        sample.url.as_deref(),
                        sample.domain.as_deref(),
                    )
                } else {
                    Ok(false)
                }
            }
            Some((app, title, kind, label, bundle_id, url)) => {
                if idle {
                    self.pending_switch = None;
                    if kind != KIND_ACTIVITY {
                        return Ok(false);
                    }
                    // The break began at the last input, not when the
                    // threshold was crossed; the time in between was idle.
                    let started_at = self.current.as_ref().map_or(now, |cur| cur.started_at);
                    let idle_since = now.saturating_sub(idle_ms).max(started_at);
                    self.close_current(idle_since)?;
                    self.manual_tracking = false;

                    // If outside window, do not open an Idle break segment
                    if !in_window {
                        return Ok(true);
                    }

                    return self.open(
                        "Idle",
                        "No activity",
                        KIND_BREAK,
                        Some(IDLE_LABEL),
                        idle_since,
                        None,
                        None,
                        None,
                    );
                }

                if kind == KIND_BREAK && label.as_deref() == Some(IDLE_LABEL) {
                    self.pending_switch = None;
                    self.close_current(now)?;
                    if !in_window && !self.manual_tracking {
                        return Ok(true);
                    }
                    return match sample {
                        Some(sample) => self.open(
                            &sample.app,
                            &sample.title,
                            KIND_ACTIVITY,
                            None,
                            now,
                            sample.bundle_id.as_deref(),
                            sample.url.as_deref(),
                            sample.domain.as_deref(),
                        ),
                        None => Ok(true),
                    };
                }

                if kind != KIND_ACTIVITY {
                    self.pending_switch = None;
                    return Ok(false);
                }

                match sample {
                    Some(sample) => {
                        let same_app = is_same_app(&app, bundle_id.as_deref(), &sample);
                        if same_app {
                            // User is in the current app (or returned to it from a brief detour).
                            self.pending_switch = None;

                            if sample.title != title
                                || sample.url != url
                                || sample.bundle_id != bundle_id
                            {
                                self.close_current(now)?;
                                self.open(
                                    &sample.app,
                                    &sample.title,
                                    KIND_ACTIVITY,
                                    None,
                                    now,
                                    sample.bundle_id.as_deref(),
                                    sample.url.as_deref(),
                                    sample.domain.as_deref(),
                                )
                            } else {
                                Ok(false)
                            }
                        } else {
                            // User is in a different app.
                            match self.pending_switch.take() {
                                None => {
                                    self.pending_switch = Some(PendingSwitch {
                                        sample,
                                        started_at: now,
                                        started_instant: Some(std::time::Instant::now()),
                                    });
                                    Ok(false)
                                }
                                Some(pending) => {
                                    let same_pending = is_same_app(
                                        &pending.sample.app,
                                        pending.sample.bundle_id.as_deref(),
                                        &sample,
                                    );
                                    if same_pending {
                                        let elapsed_clock = now.saturating_sub(pending.started_at);
                                        let elapsed_instant = pending
                                            .started_instant
                                            .map_or(0, |i| i.elapsed().as_millis() as u64);
                                        let elapsed = elapsed_clock.max(elapsed_instant);

                                        if elapsed >= APP_SWITCH_GRACE_MS {
                                            self.close_current(pending.started_at)?;
                                            self.open(
                                                &sample.app,
                                                &sample.title,
                                                KIND_ACTIVITY,
                                                None,
                                                pending.started_at,
                                                sample.bundle_id.as_deref(),
                                                sample.url.as_deref(),
                                                sample.domain.as_deref(),
                                            )
                                        } else {
                                            self.pending_switch = Some(PendingSwitch {
                                                sample,
                                                started_at: pending.started_at,
                                                started_instant: pending.started_instant,
                                            });
                                            Ok(false)
                                        }
                                    } else {
                                        // A -> B -> C burst: previous detour aborted without fragmenting
                                        self.pending_switch = Some(PendingSwitch {
                                            sample,
                                            started_at: now,
                                            started_instant: Some(std::time::Instant::now()),
                                        });
                                        Ok(false)
                                    }
                                }
                            }
                        }
                    }
                    None => Ok(false),
                }
            }
        }
    }

    pub fn start_session(
        &mut self,
        kind: &str,
        label: Option<&str>,
        now: u64,
    ) -> Result<bool, String> {
        if kind != KIND_FOCUS && kind != KIND_MEETING && kind != KIND_BREAK {
            return Err(format!("cannot start a session of kind {kind}"));
        }
        self.pending_switch = None;
        self.manual_tracking = true;
        self.close_current(now)?;
        let display = label.map(str::trim).filter(|text| !text.is_empty());
        let app = match kind {
            KIND_FOCUS => "Focus",
            KIND_MEETING => "Meeting",
            _ => "Break",
        };
        let title = display.unwrap_or(app);
        self.open(app, title, kind, display, now, None, None, None)
    }

    pub fn stop_session(&mut self, now: u64) -> Result<bool, String> {
        self.pending_switch = None;
        self.manual_tracking = false;
        self.close_current(now)
    }

    /// Nobody is at the Mac (see `capture::user_is_away`): end the activity
    /// segment now, not at the last input, and treat the time as idle until
    /// the user is back.
    pub fn tick_away(&mut self, now: u64) -> Result<bool, String> {
        self.pending_switch = None;
        let closed = if self
            .current
            .as_ref()
            .is_some_and(|cur| cur.kind == KIND_ACTIVITY)
        {
            self.close_active_segment(now)?
        } else {
            false
        };
        let threshold = self.idle_threshold_ms;
        Ok(self.tick(None, threshold, now)? || closed)
    }

    pub fn close_active_segment(&mut self, now: u64) -> Result<bool, String> {
        self.pending_switch = None;
        self.manual_tracking = false;
        self.close_current(now)
    }

    pub fn set_capture_enabled(&mut self, enabled: bool, now: u64) -> Result<(), String> {
        self.pending_switch = None;
        if enabled {
            self.capture_enabled = true;
            self.write_setting("capture_enabled", "true")?;
            if !self.is_in_tracking_window(now) {
                self.manual_tracking = true;
            }
        } else {
            if self.manual_tracking && !self.is_in_tracking_window(now) {
                self.manual_tracking = false;
                self.close_current(now)?;
            } else {
                self.manual_tracking = false;
                self.capture_enabled = false;
                self.write_setting("capture_enabled", "false")?;
                self.close_current(now)?;
            }
        }
        Ok(())
    }

    pub fn set_idle_threshold_ms(&mut self, ms: u64) -> Result<(), String> {
        self.idle_threshold_ms = ms.max(SAMPLE_SECS * 1000);
        self.write_setting("idle_threshold_ms", &self.idle_threshold_ms.to_string())
    }

    pub fn mark_reviewed(&mut self, id: i64) -> Result<(), String> {
        if let Some(path) = self.database_path.as_ref() {
            let database = crate::database::connect(path).map_err(|error| error.to_string())?;
            crate::database::repositories::activity::ActivityRepository::new(&database)
                .mark_reviewed(id)
                .map_err(|error| error.to_string())?;
            if let Some(current) = &mut self.current
                && current.id == id
            {
                current.reviewed = true;
            }
            return Ok(());
        }
        #[cfg(not(test))]
        {
            Err("database path unavailable for segment update".into())
        }
        #[cfg(test)]
        {
            self.conn
                .execute(
                    "UPDATE segments SET reviewed = 1 WHERE id = ?1",
                    params![id],
                )
                .map_err(|error| error.to_string())?;
            if let Some(current) = &mut self.current
                && current.id == id
            {
                current.reviewed = true;
            }
            Ok(())
        }
    }

    pub fn purge_older_than(&mut self, days: u32, now: u64) -> Result<u64, String> {
        if days == 0 {
            return Ok(0);
        }
        let cutoff = now.saturating_sub(u64::from(days) * ONE_DAY_MS);
        if let Some(path) = self.database_path.as_ref() {
            let database = crate::database::connect(path).map_err(|error| error.to_string())?;
            return crate::database::repositories::activity::ActivityRepository::new(&database)
                .purge_older_than(cutoff as i64, (cutoff / ONE_DAY_MS) as i64)
                .map_err(|error| error.to_string());
        }
        #[cfg(not(test))]
        return Err("database path unavailable for activity purge".into());
        #[cfg(test)]
        let transaction = self.conn.transaction().map_err(|error| error.to_string())?;
        #[cfg(test)]
        let removed = transaction
            .execute(
                "DELETE FROM segments WHERE ended_at IS NOT NULL AND ended_at < ?1",
                params![cutoff as i64],
            )
            .map_err(|error| error.to_string())?;
        #[cfg(test)]
        transaction
            .execute(
                "DELETE FROM breaks WHERE created_at < ?1",
                params![cutoff as i64],
            )
            .map_err(|error| error.to_string())?;
        #[cfg(test)]
        transaction
            .execute(
                "DELETE FROM daily_rollups WHERE day_epoch < ?1",
                params![(cutoff / ONE_DAY_MS) as i64],
            )
            .map_err(|error| error.to_string())?;
        #[cfg(test)]
        transaction.commit().map_err(|error| error.to_string())?;
        #[cfg(test)]
        Ok(removed as u64)
    }

    pub fn live_state(&self, now: u64) -> LiveState {
        let in_tracking_hours = self.tracking_hours.is_inside_window(now);
        let tracking_active = self.capture_enabled
            && (in_tracking_hours
                || self.manual_tracking
                || self
                    .current
                    .as_ref()
                    .is_some_and(|c| c.kind == KIND_ACTIVITY));
        LiveState {
            current: self.current.clone(),
            capture_enabled: self.capture_enabled,
            idle_threshold_ms: self.idle_threshold_ms,
            last_idle_ms: self.last_idle_ms,
            watch_since_ms: self.watch_since_ms,
            in_tracking_hours,
            tracking_active,
        }
    }

    pub fn snapshot(&self, since_ms: u64, now: u64) -> Result<ActivitySnapshot, String> {
        if let Some(path) = self.database_path.as_ref() {
            let database = crate::database::connect(path).map_err(|error| error.to_string())?;
            let (mut segments, mut totals) =
                crate::database::repositories::activity::ActivityRepository::new(&database)
                    .snapshot_data(since_ms as i64, now as i64)
                    .map_err(|error| error.to_string())?;
            add_current_totals(&mut totals, self.current.as_ref(), since_ms, now);
            if let Some(current) = &self.current
                && !segments.iter().any(|segment| segment.id == current.id)
            {
                segments.push(current.as_segment());
            }
            return Ok(ActivitySnapshot {
                current: self.current.as_ref().map(Current::as_segment),
                segments,
                tracked_ms: totals.tracked_ms,
                focus_ms: totals.focus_ms,
                break_ms: totals.break_ms,
                unreviewed: totals.unreviewed,
                idle_ms: self.last_idle_ms,
                idle_threshold_ms: self.idle_threshold_ms,
                capture_enabled: self.capture_enabled,
                in_tracking_hours: self.is_in_tracking_window(now),
                tracking_active: self.live_state(now).tracking_active,
            });
        }
        #[cfg(not(test))]
        return Err("database path unavailable for activity snapshot".into());
        #[cfg(test)]
        build_snapshot(&self.conn, &self.live_state(now), since_ms, now)
    }

    pub fn tick_summary(&self, now: u64) -> Result<ActivityTick, String> {
        let live = self.live_state(now);
        let since_ms = live.watch_since_ms.unwrap_or(0);
        if let Some(path) = self.database_path.as_ref() {
            let database = crate::database::connect(path).map_err(|error| error.to_string())?;
            let (_, mut totals) =
                crate::database::repositories::activity::ActivityRepository::new(&database)
                    .snapshot_data(since_ms as i64, now as i64)
                    .map_err(|error| error.to_string())?;
            add_current_totals(&mut totals, self.current.as_ref(), since_ms, now);
            return Ok(ActivityTick {
                current: self.current.as_ref().map(Current::as_segment),
                tracked_ms: totals.tracked_ms,
                focus_ms: totals.focus_ms,
                break_ms: totals.break_ms,
                unreviewed: totals.unreviewed,
                idle_ms: live.last_idle_ms,
                idle_threshold_ms: live.idle_threshold_ms,
                capture_enabled: live.capture_enabled,
                in_tracking_hours: live.in_tracking_hours,
                tracking_active: live.tracking_active,
            });
        }
        #[cfg(not(test))]
        return Err("database path unavailable for activity tick".into());
        #[cfg(test)]
        build_tick(&self.conn, &live, since_ms, now)
    }

    #[allow(clippy::too_many_arguments)]
    fn open(
        &mut self,
        app: &str,
        title: &str,
        kind: &str,
        label: Option<&str>,
        now: u64,
        bundle_id: Option<&str>,
        url: Option<&str>,
        domain: Option<&str>,
    ) -> Result<bool, String> {
        let segment_id = if let Some(path) = self.database_path.as_ref() {
            let database = crate::database::connect(path).map_err(|error| error.to_string())?;
            crate::database::repositories::activity::ActivityRepository::new(&database)
                .open_segment(crate::database::repositories::activity::NewSegment {
                    app,
                    title,
                    kind,
                    label,
                    started_at: now as i64,
                    bundle_id,
                    url,
                    domain,
                })
                .map_err(|error| error.to_string())?
        } else {
            #[cfg(not(test))]
            return Err("database path unavailable for segment creation".into());
            #[cfg(test)]
            {
                self.conn
            .execute(
                "INSERT INTO segments (app, title, kind, label, started_at, ended_at, reviewed, bundle_id, url, domain)
                 VALUES (?1, ?2, ?3, ?4, ?5, NULL, 0, ?6, ?7, ?8)",
                params![app, title, kind, label, now as i64, bundle_id, url, domain],
            )
            .map_err(|error| error.to_string())?;
                self.conn.last_insert_rowid()
            }
        };

        // Browser tabs are tracked per domain, not per browser app, so a
        // domain (when present) takes priority over the bundle id as the
        // identifier; errors here are swallowed because a failed apps-table
        // upsert must not stop the segment itself from being recorded.
        if kind == KIND_ACTIVITY {
            let identifier = domain.unwrap_or(bundle_id.unwrap_or(app));
            let app_kind = if domain.is_some() { "site" } else { "app" };
            if let Some(path) = self.database_path.as_ref()
                && let Ok(database) = crate::database::connect(path)
            {
                let _ = crate::database::repositories::apps::AppRepository::new(&database)
                    .observe(app_kind, identifier, app, now);
            }
            #[cfg(test)]
            if self.database_path.is_none() {
                let app_id = uuid::Uuid::now_v7().to_string();
                let _ = self.conn.execute(
                    "INSERT INTO apps (id, kind, identifier, display_name, excluded, first_seen, last_seen, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, 0, ?5, ?5, ?5, ?5)
                     ON CONFLICT(identifier) DO UPDATE SET last_seen = ?5, updated_at = ?5;",
                    params![app_id, app_kind, identifier, app, now as i64],
                );
            }
        }

        self.current = Some(Current {
            id: segment_id,
            app: app.to_string(),
            title: title.to_string(),
            kind: kind.to_string(),
            label: label.map(str::to_string),
            started_at: now,
            reviewed: false,
            app_id: None,
            bundle_id: bundle_id.map(str::to_string),
            url: url.map(str::to_string),
            domain: domain.map(str::to_string),
            entry_id: None,
        });
        Ok(true)
    }

    pub(crate) fn close_current(&mut self, now: u64) -> Result<bool, String> {
        let Some(current) = self.current.take() else {
            return Ok(false);
        };
        if let Some(path) = self.database_path.as_ref() {
            let database = crate::database::connect(path).map_err(|error| error.to_string())?;
            crate::database::repositories::activity::ActivityRepository::new(&database)
                .close_segment(current.id, now as i64)
                .map_err(|error| error.to_string())?;
            return Ok(true);
        }
        #[cfg(not(test))]
        {
            Err("database path unavailable for segment close".into())
        }
        #[cfg(test)]
        {
            self.conn
                .execute(
                    "UPDATE segments SET ended_at = ?1 WHERE id = ?2",
                    params![now as i64, current.id],
                )
                .map_err(|error| error.to_string())?;
            Ok(true)
        }
    }

    // --- P1: Categories CRUD --------------------------------------------
}
