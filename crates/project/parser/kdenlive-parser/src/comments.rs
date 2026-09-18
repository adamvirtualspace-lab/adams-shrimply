use super::*;
use shrimply_project_document::{CommentColor, CommentItem, CommentTrack};

// Kdenlive's MarkerListModel::toJson stores pos and duration in project frames.
// Categories are stored separately as {index, comment, color} in the main bin.
const LEGACY_COLORS: [CommentColor; 9] = [
    CommentColor::Purple,
    CommentColor::Blue,
    CommentColor::Green,
    CommentColor::Green,
    CommentColor::Yellow,
    CommentColor::Yellow,
    CommentColor::Orange,
    CommentColor::Orange,
    CommentColor::Red,
];

impl Converter<'_> {
    pub(super) fn comment_tracks(
        &mut self,
        active: &Element,
        main_bin: &Element,
    ) -> Result<Vec<CommentTrack>, Box<dyn Error + Send + Sync>> {
        let mut colors = HashMap::new();
        if let Some(data) = main_bin
            .property("kdenlive:docproperties.guidesCategories")
            .filter(|data| !data.trim().is_empty())
        {
            for category in serde_json::from_str::<Vec<Value>>(data)? {
                let index = category
                    .get("index")
                    .and_then(Value::as_i64)
                    .ok_or_else(|| invalid("marker category has no integer index"))?;
                let hex = category
                    .get("color")
                    .and_then(Value::as_str)
                    .and_then(|color| color.strip_prefix('#'))
                    .filter(|hex| hex.len() == 6)
                    .ok_or_else(|| invalid("marker category color must be #RRGGBB"))?;
                let rgb = Color::<u8>::from_rgb_u32(u32::from_str_radix(hex, 16)?);
                colors.insert(index, math::comment_color(rgb));
            }
        }
        // Timeline producers (including timewarp and audio clones) share a bin ID.
        let bin_markers = main_bin
            .children_named("entry")
            .filter_map(|entry| entry.attribute("producer"))
            .filter_map(|id| self.index.get(id).copied())
            .filter_map(|producer| {
                Some((
                    producer.property("kdenlive:id")?,
                    producer.property("kdenlive:markers")?,
                ))
            })
            .collect::<HashMap<_, _>>();
        let mut items =
            self.sequence_comments(active, &colors, &bin_markers, &mut HashSet::new())?;
        if active
            .property("kdenlive:sequenceproperties.guides")
            .is_none()
        {
            items.extend(markers(
                main_bin.property("kdenlive:docproperties.guides"),
                &colors,
                self.fps,
            )?);
        }
        let step = frame_time(1, self.fps);
        for item in &mut items {
            item.start = item.start.snapped(step);
            item.end = item.end.snapped(step).max(item.start.saturating_add(step));
        }
        items.sort_by(|left, right| {
            (left.start, left.end, &left.text, left.color.label()).cmp(&(
                right.start,
                right.end,
                &right.text,
                right.color.label(),
            ))
        });
        // A/V instances of the same bin clip expose the same markers twice.
        items.dedup_by(|left, right| {
            left.start == right.start
                && left.end == right.end
                && left.text == right.text
                && left.color == right.color
        });
        let mut tracks: Vec<CommentTrack> = Vec::new();
        for item in items {
            if let Some(track) = tracks
                .iter_mut()
                .find(|track| track.items.last().is_none_or(|last| last.end <= item.start))
            {
                track.items.push(item);
            } else {
                tracks.push(CommentTrack {
                    items: vec![item],
                    ..Default::default()
                });
            }
        }
        if !tracks.is_empty() {
            self.warnings.insert("Kdenlive markers were imported as independent comment items. Point markers use one frame; category colors use Shrimply's named palette. Clip and nested-sequence markers no longer follow their source clips.".to_owned());
        }
        Ok(tracks)
    }

    fn sequence_comments(
        &mut self,
        tractor: &Element,
        colors: &HashMap<i64, CommentColor>,
        bin_markers: &HashMap<&str, &str>,
        visiting: &mut HashSet<Uuid>,
    ) -> Result<Vec<CommentItem>, Box<dyn Error + Send + Sync>> {
        let id = parse_uuid(
            tractor
                .property("kdenlive:uuid")
                .ok_or_else(|| invalid("sequence has no UUID"))?,
        )?;
        if !visiting.insert(id) {
            return Err(invalid("cyclic Kdenlive sequence references"));
        }
        let mut result = markers(
            tractor.property("kdenlive:sequenceproperties.guides"),
            colors,
            self.fps,
        )?;
        let lanes = tractor
            .children_named("track")
            .enumerate()
            .filter(|(position, _)| *position != 0)
            .filter_map(|(_, track)| track.attribute("producer"))
            .filter_map(|id| self.index.get(id).copied())
            .filter(|track| {
                !matches!(
                    track.property("kdenlive:playlistid"),
                    Some("black_track" | "timeline_preview")
                )
            })
            .flat_map(|track| track.children_named("track"))
            .filter_map(|track| track.attribute("producer"))
            .filter_map(|id| self.index.get(id).copied())
            .collect::<Vec<_>>();
        for lane in lanes {
            let mut cursor = 0_i64;
            for node in &lane.children {
                if !matches!(node.name.as_str(), "entry" | "blank") {
                    continue;
                }
                let duration = element_duration(node, self.fps)?;
                let end = cursor
                    .checked_add(duration)
                    .ok_or_else(|| invalid("marker timeline position overflowed"))?;
                if node.name == "entry" {
                    let producer = self.entry_producer(node)?;
                    let data = producer
                        .property("kdenlive:id")
                        .and_then(|id| bin_markers.get(id).copied())
                        .or_else(|| producer.property("kdenlive:markers"));
                    let mut comments = markers(data, colors, self.fps)?;
                    if producer.name == "tractor" {
                        comments.extend(self.sequence_comments(
                            producer,
                            colors,
                            bin_markers,
                            visiting,
                        )?);
                    }
                    if !comments.is_empty() {
                        let source = self.source(producer)?;
                        let offset = entry_in(node, self.fps)?;
                        let source_frame = match source.reverse_origin_frame {
                            Some(origin) => origin
                                .checked_sub(offset)
                                .ok_or_else(|| invalid("marker source position overflowed"))?,
                            None => offset,
                        };
                        let source_start = source_time(source_frame, source.speed.abs(), self.fps);
                        if source.speed == Fraction::from(0_u64) {
                            return Err(invalid("marker clip has zero playback speed"));
                        }
                        for mut item in comments {
                            if let Some((start, finish)) = math::project_marker_range(
                                (item.start, item.end),
                                (frame_time(cursor, self.fps), frame_time(end, self.fps)),
                                source_start,
                                source.speed,
                                frame_time(1, self.fps),
                            ) {
                                item.start = start;
                                item.end = finish;
                                result.push(item);
                            }
                        }
                    }
                }
                cursor = end;
            }
        }
        visiting.remove(&id);
        Ok(result)
    }
}

fn markers(
    data: Option<&str>,
    colors: &HashMap<i64, CommentColor>,
    fps: Fraction,
) -> Result<Vec<CommentItem>, Box<dyn Error + Send + Sync>> {
    let Some(data) = data.filter(|data| !data.trim().is_empty()) else {
        return Ok(Vec::new());
    };
    serde_json::from_str::<Vec<Value>>(data)?
        .into_iter()
        .map(|marker| {
            let pos = marker
                .get("pos")
                .and_then(Value::as_i64)
                .filter(|pos| *pos >= 0)
                .ok_or_else(|| invalid("marker position must be a nonnegative frame number"))?;
            let duration = marker
                .get("duration")
                .map_or(Some(0), Value::as_i64)
                .filter(|duration| *duration >= 0)
                .ok_or_else(|| invalid("marker duration must be a nonnegative frame count"))?;
            let end = pos
                .checked_add(duration)
                .ok_or_else(|| invalid("marker range overflowed"))?;
            let kind = marker
                .get("type")
                .map_or(Some(0), Value::as_i64)
                .ok_or_else(|| invalid("marker category must be an integer"))?;
            let text = marker
                .get("comment")
                .map_or(Some("Marker"), Value::as_str)
                .ok_or_else(|| invalid("marker comment must be text"))?;
            let mut item =
                CommentItem::new(frame_time(pos, fps), frame_time(end, fps), text.to_owned());
            item.color = colors
                .get(&kind)
                .copied()
                .unwrap_or(LEGACY_COLORS[kind.rem_euclid(LEGACY_COLORS.len() as i64) as usize]);
            Ok(item)
        })
        .collect()
}
