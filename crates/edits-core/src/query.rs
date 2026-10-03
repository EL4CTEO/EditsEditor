//! Locating objects in a project by id.

use crate::model::{Clip, Composition, EffectInstance, Project, Track};

/// Where a clip lives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClipLoc {
    pub comp: String,
    pub track: usize,
    pub clip: usize,
}

/// Where a track lives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrackLoc {
    pub comp: String,
    pub track: usize,
}

/// Where an effect instance lives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EffectOwner {
    Comp(String),
    Track(TrackLoc),
    Clip(ClipLoc),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectLoc {
    pub owner: EffectOwner,
    pub index: usize,
}

/// Kind of object an id refers to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ObjectRef {
    Comp(String),
    Asset(String),
    Track(TrackLoc),
    Clip(ClipLoc),
    Effect(EffectLoc),
    Marker { comp: String, index: usize },
}

impl ObjectRef {
    pub fn kind(&self) -> &'static str {
        match self {
            ObjectRef::Comp(_) => "composition",
            ObjectRef::Asset(_) => "asset",
            ObjectRef::Track(_) => "track",
            ObjectRef::Clip(_) => "clip",
            ObjectRef::Effect(_) => "effect",
            ObjectRef::Marker { .. } => "marker",
        }
    }
}

impl Project {
    pub fn find(&self, id: &str) -> Option<ObjectRef> {
        if self.compositions.contains_key(id) {
            return Some(ObjectRef::Comp(id.to_string()));
        }
        if self.assets.contains_key(id) {
            return Some(ObjectRef::Asset(id.to_string()));
        }
        for (cid, comp) in &self.compositions {
            if let Some(i) = comp.effects.iter().position(|e| e.id == id) {
                return Some(ObjectRef::Effect(EffectLoc { owner: EffectOwner::Comp(cid.clone()), index: i }));
            }
            if let Some(i) = comp.markers.iter().position(|m| m.id == id) {
                return Some(ObjectRef::Marker { comp: cid.clone(), index: i });
            }
            for (ti, track) in comp.tracks.iter().enumerate() {
                let tl = TrackLoc { comp: cid.clone(), track: ti };
                if track.id == id {
                    return Some(ObjectRef::Track(tl));
                }
                if let Some(i) = track.effects.iter().position(|e| e.id == id) {
                    return Some(ObjectRef::Effect(EffectLoc { owner: EffectOwner::Track(tl), index: i }));
                }
                for (ci, clip) in track.clips.iter().enumerate() {
                    let cl = ClipLoc { comp: cid.clone(), track: ti, clip: ci };
                    if clip.id == id {
                        return Some(ObjectRef::Clip(cl));
                    }
                    if let Some(i) = clip.effects.iter().position(|e| e.id == id) {
                        return Some(ObjectRef::Effect(EffectLoc { owner: EffectOwner::Clip(cl), index: i }));
                    }
                }
            }
        }
        None
    }

    pub fn find_clip(&self, id: &str) -> Option<ClipLoc> {
        for (cid, comp) in &self.compositions {
            for (ti, track) in comp.tracks.iter().enumerate() {
                if let Some(ci) = track.clips.iter().position(|c| c.id == id) {
                    return Some(ClipLoc { comp: cid.clone(), track: ti, clip: ci });
                }
            }
        }
        None
    }

    pub fn find_track(&self, id: &str) -> Option<TrackLoc> {
        for (cid, comp) in &self.compositions {
            if let Some(ti) = comp.tracks.iter().position(|t| t.id == id) {
                return Some(TrackLoc { comp: cid.clone(), track: ti });
            }
        }
        None
    }

    pub fn clip(&self, id: &str) -> Option<&Clip> {
        let l = self.find_clip(id)?;
        self.clip_at(&l)
    }

    pub fn clip_mut(&mut self, id: &str) -> Option<&mut Clip> {
        let l = self.find_clip(id)?;
        self.clip_at_mut(&l)
    }

    pub fn clip_at(&self, l: &ClipLoc) -> Option<&Clip> {
        self.compositions.get(&l.comp)?.tracks.get(l.track)?.clips.get(l.clip)
    }

    pub fn clip_at_mut(&mut self, l: &ClipLoc) -> Option<&mut Clip> {
        self.compositions.get_mut(&l.comp)?.tracks.get_mut(l.track)?.clips.get_mut(l.clip)
    }

    pub fn track(&self, id: &str) -> Option<&Track> {
        let l = self.find_track(id)?;
        self.compositions.get(&l.comp)?.tracks.get(l.track)
    }

    pub fn track_mut(&mut self, id: &str) -> Option<&mut Track> {
        let l = self.find_track(id)?;
        self.compositions.get_mut(&l.comp)?.tracks.get_mut(l.track)
    }

    /// Composition that contains the clip/track/effect with this id (or the comp itself).
    pub fn comp_of(&self, id: &str) -> Option<&Composition> {
        let cid = match self.find(id)? {
            ObjectRef::Comp(c) => c,
            ObjectRef::Track(t) => t.comp,
            ObjectRef::Clip(c) => c.comp,
            ObjectRef::Effect(e) => match e.owner {
                EffectOwner::Comp(c) => c,
                EffectOwner::Track(t) => t.comp,
                EffectOwner::Clip(c) => c.comp,
            },
            ObjectRef::Marker { comp, .. } => comp,
            ObjectRef::Asset(_) => return None,
        };
        self.compositions.get(&cid)
    }

    pub fn effect_list_mut(&mut self, owner: &EffectOwner) -> Option<&mut Vec<EffectInstance>> {
        match owner {
            EffectOwner::Comp(c) => self.compositions.get_mut(c).map(|c| &mut c.effects),
            EffectOwner::Track(t) => self.compositions.get_mut(&t.comp)?.tracks.get_mut(t.track).map(|t| &mut t.effects),
            EffectOwner::Clip(c) => self.clip_at_mut(c).map(|c| &mut c.effects),
        }
    }

    pub fn effect_list(&self, owner: &EffectOwner) -> Option<&Vec<EffectInstance>> {
        match owner {
            EffectOwner::Comp(c) => self.compositions.get(c).map(|c| &c.effects),
            EffectOwner::Track(t) => self.compositions.get(&t.comp)?.tracks.get(t.track).map(|t| &t.effects),
            EffectOwner::Clip(c) => self.clip_at(c).map(|c| &c.effects),
        }
    }

    /// Iterate over all clips with their composition id and track index.
    pub fn all_clips(&self) -> impl Iterator<Item = (&str, usize, &Clip)> {
        self.compositions.iter().flat_map(|(cid, comp)| {
            comp.tracks
                .iter()
                .enumerate()
                .flat_map(move |(ti, t)| t.clips.iter().map(move |c| (cid.as_str(), ti, c)))
        })
    }

    /// The clip on the same track that precedes `loc` in time (ends at or after this clip's start - eps).
    pub fn previous_clip(&self, loc: &ClipLoc) -> Option<&Clip> {
        let track = self.compositions.get(&loc.comp)?.tracks.get(loc.track)?;
        let me = track.clips.get(loc.clip)?;
        track
            .clips
            .iter()
            .filter(|c| c.id != me.id && c.enabled && c.start < me.start)
            .max_by(|a, b| a.end().total_cmp(&b.end()))
    }
}
