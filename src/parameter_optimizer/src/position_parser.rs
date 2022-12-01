use std::collections::{HashMap, HashSet};
use std::net::IpAddr;

use crate::util;

pub use glam::Vec3A as Vec3;
pub type UavId = IpAddr;

#[derive(Debug, PartialEq, PartialOrd, Clone, Copy)]
pub struct TimePoint(pub f32);

impl TimePoint {
    fn is_after(&self, other: &TimePoint) -> bool {
        self >= other
    }

    fn is_before(&self, other: &TimePoint) -> bool {
        self <= other
    }
}

/// Data that is recorded at a point in time during the simulation
#[derive(Debug, PartialEq)]
pub struct UavKeyFrame {
    ip: UavId,
    pos: Vec3,
}

#[derive(Debug, PartialEq)]
pub enum Event {
    ColorChange((UavId, Vec3)),
}

#[derive(Debug, PartialEq, Eq)]
enum InterpolationState {
    /// Indicates that we are before the first data point. Value is the index into frames
    Before(usize),

    /// Indicates that there are frames in time that surround this time point
    /// Value is the index into frames of the first one. The second one resides at index + 1
    Interpolate(usize, usize),
    /// Indicates that the frame is before now and there are no more frames for this uav in the
    /// future
    After(usize),
}

#[derive(Debug, PartialEq)]
struct TimedObject<T> {
    time: TimePoint,
    inner: T,
}

impl<T> TimedObject<T> {
    fn new(time: f32, inner: T) -> Self {
        Self {
            time: TimePoint(time),
            inner,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct SimulationData {
    frames: Vec<TimedObject<HashMap<UavId, UavKeyFrame>>>,
    state: HashMap<UavId, InterpolationState>,
    events: Vec<TimedObject<Event>>,
    last_time: Option<TimePoint>,

    pub uavs: HashSet<IpAddr>,
    pub simulation_length: f32,
}

impl SimulationData {
    pub fn parse(data: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mut frames = Vec::new();
        let mut events = Vec::new();
        let mut last_time = None;
        let mut inner_map = HashMap::new();
        let mut unique_ids = HashSet::new();
        for line in data.lines().skip(1) {
            if line.starts_with("color") {
                // Color directive
                let (time, ip, r, g, b) = scan_fmt::scan_fmt!(
                    line,
                    "color,{f},{},{f},{f},{f}",
                    f32,
                    IpAddr,
                    f32,
                    f32,
                    f32
                )?;
                events.push(TimedObject::new(
                    time,
                    Event::ColorChange((ip, Vec3::new(r, g, b))),
                ));
            } else {
                //Normal pos line
                let (time, ip, x, y, z) =
                    scan_fmt::scan_fmt!(line, "{f},{},{f},{f},{f}", f32, IpAddr, f32, f32, f32)?;
                unique_ids.insert(ip);
                //Finish last part of frame
                if let Some(last_time) = last_time {
                    if last_time != time {
                        //Finish inner map
                        frames.push(TimedObject::new(last_time, std::mem::take(&mut inner_map)));
                    }
                }
                last_time = Some(time);
                inner_map.insert(
                    ip,
                    UavKeyFrame {
                        ip,
                        pos: Vec3::new(x, y, z),
                    },
                );
            }
        }
        if let Some(last_time) = last_time {
            //Finish the last inner map
            frames.push(TimedObject::new(last_time, std::mem::take(&mut inner_map)));
        }

        //Set the initial state. Because we assume the user starts the simulation at t=0, all the UAV's
        //will be in the before state because we only know their position in the future
        let mut state = HashMap::new();
        if !frames.is_empty() {
            for uav in &unique_ids {
                for (i, _) in frames.iter().enumerate() {
                    let entry = &frames[i];
                    if entry.inner.contains_key(uav) {
                        state.insert(*uav, InterpolationState::Before(i));
                        break;
                    }
                }
            }
        }
        let simulation_length = frames[frames.len() - 1].time.0;
        Ok(Self {
            frames,
            state,
            events,
            last_time: None,
            simulation_length,
            uavs: unique_ids,
        })
    }

    /// Returns the position of the specified UAV at the given point in time
    ///
    /// time must never decrease from one call of this function to the next
    pub fn pos_at_time(&mut self, now: TimePoint, uav: UavId) -> Option<Vec3> {
        if let Some(last_time) = self.last_time {
            assert!(now.is_after(&last_time));
        }
        self.last_time = Some(now);

        let start_index = match self.state.get(&uav) {
            Some(state) => match &state {
                InterpolationState::Before(index) => {
                    //Last time we were before the old value but we still need to check to see if
                    //we've passed the old one
                    let old_entry = &self.frames[*index];
                    if now.is_before(&old_entry.time) {
                        //We are still before the old value
                        return Some(old_entry.inner.get(&uav).unwrap().pos);
                    } else {
                        *index
                    }
                }
                InterpolationState::Interpolate(a, b) => {
                    if self.frames[*b].time.is_before(&now) {
                        *b
                    } else {
                        *a
                    }
                }
                InterpolationState::After(index) => {
                    //Easy - we know there are no more values after this one
                    return Some(self.frames[*index].inner.get(&uav).unwrap().pos);
                }
            },
            None => return None,
        };
        //We are either before or in between two key frames check where the new time unit falls then interpolate
        let mut last_index = start_index;
        for i in (start_index + 1)..self.frames.len() {
            let entry = &self.frames[i];
            if let Some(new_entry) = entry.inner.get(&uav) {
                last_index = i;
                if entry.time.is_after(&now) {
                    //We found an entry which is after this event
                    //Make sure the last entry is before now
                    let last_frame = &self.frames[start_index];
                    assert!(last_frame.time.is_before(&now));
                    assert!(last_frame.inner.contains_key(&uav));
                    let a_pos = last_frame.inner.get(&uav).unwrap().pos;
                    let a_time = last_frame.time;
                    let b_pos = new_entry.pos;
                    let b_time = entry.time;

                    let pos = util::map(a_time.0, b_time.0, now.0, a_pos, b_pos);

                    self.state
                        .insert(uav, InterpolationState::Interpolate(start_index, i));

                    return Some(pos);
                }
            }
        }
        //If we got to the end without finding an upper bound, that means we are at the end of the
        //last data point. Switch to after state and return last data point
        self.state
            .insert(uav, InterpolationState::After(last_index));

        Some(self.frames[last_index].inner.get(&uav).unwrap().pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic() {
        let data = SimulationData::parse(
            r#"Time (s),IP Address, X (m), Y (m), Z (m)
0,10.1.1.1,0,0,0,
0,10.1.1.2,-2.9477,-0.330799,-2.24833,
0,10.1.1.3,1.43092,3.47754,0.155331,
0,10.1.1.4,0.21543,1.23135,1.60952,
0,10.1.1.5,-0.508709,-0.178146,-1.80075,
0,10.1.1.6,0.0361832,-1.44774,-0.0481865,
0,10.1.1.7,2.16164,-2.99708,1.50764,
0,10.1.1.8,1.03635,1.8033,3.10858,
color,0,10.1.1.1,0.3,0.7,1,
0.05,10.1.1.1,0,0,0,"#,
        )
        .unwrap();

        assert_eq!(data.frames.len(), 2);
        assert_eq!(data.frames[0].inner.len(), 8);
        assert_eq!(data.frames[0].time, TimePoint(0f32));
        assert_eq!(data.frames[1].inner.len(), 1);
        assert_eq!(data.frames[1].time, TimePoint(0.05f32));
        assert_eq!(data.events.len(), 1);

        assert_eq!(
            data.events,
            vec!(TimedObject::new(
                0f32,
                Event::ColorChange(("10.1.1.1".parse().unwrap(), [0.3, 0.7, 1.0].into()))
            ))
        );
    }

    macro_rules! assert_approx_eq {
        ($a:expr, $b:expr) => {{
            let eps = 1.0e-5;
            let (a, b) = (&$a, &$b);
            assert!(
                (*a - *b).length() < eps,
                "assertion failed: `(left !== right)` \
             (left: `{:?}`, right: `{:?}`, expect diff: `{:?}`, real diff: `{:?}`)",
                *a,
                *b,
                eps,
                (*a - *b).abs()
            );
        }};
        ($a:expr, $b:expr, $eps:expr) => {{
            let (a, b) = (&$a, &$b);
            let eps = $eps;
            assert!(
                (*a - *b).abs() < eps,
                "assertion failed: `(left !== right)` \
             (left: `{:?}`, right: `{:?}`, expect diff: `{:?}`, real diff: `{:?}`)",
                *a,
                *b,
                eps,
                (*a - *b).abs()
            );
        }};
    }

    #[test]
    fn pos() {
        let uav = "10.1.1.1".parse().unwrap();
        let mut data = SimulationData::parse(
            r#"Time (s),IP Address, X (m), Y (m), Z (m)
0.05,10.1.1.1,0,0,0,
0.1,10.1.1.1,1,1,1,
0.2,10.1.1.1,2,2,2,
0.3,10.1.1.1,3,3,3,
0.4,10.1.1.1,-50,-8,2,
0.5,10.1.1.1,0,0,0,"#,
        )
        .unwrap();

        assert_eq!(data.frames.len(), 6);
        for frame in &data.frames {
            assert_eq!(frame.inner.len(), 1);
        }
        assert_approx_eq!(
            data.pos_at_time(TimePoint(0.0), uav).unwrap(),
            Vec3::new(0.0, 0.0, 0.0)
        );

        assert_approx_eq!(
            data.pos_at_time(TimePoint(0.0), uav).unwrap(),
            Vec3::new(0.0, 0.0, 0.0)
        );

        assert_approx_eq!(
            data.pos_at_time(TimePoint(0.01), uav).unwrap(),
            Vec3::new(0.0, 0.0, 0.0)
        );
        assert_approx_eq!(
            data.pos_at_time(TimePoint(0.03), uav).unwrap(),
            Vec3::new(0.0, 0.0, 0.0)
        );
        assert_approx_eq!(
            data.pos_at_time(TimePoint(0.05), uav).unwrap(),
            Vec3::new(0.0, 0.0, 0.0)
        );

        assert_approx_eq!(
            data.pos_at_time(TimePoint(0.05), uav).unwrap(),
            Vec3::new(0.0, 0.0, 0.0)
        );
        assert_approx_eq!(
            data.pos_at_time(TimePoint(0.1), uav).unwrap(),
            Vec3::new(1.0, 1.0, 1.0)
        );
        assert_approx_eq!(
            data.pos_at_time(TimePoint(0.15), uav).unwrap(),
            Vec3::new(1.5, 1.5, 1.5)
        );
        assert_approx_eq!(
            data.pos_at_time(TimePoint(0.2), uav).unwrap(),
            Vec3::new(2.0, 2.0, 2.0)
        );
        assert_approx_eq!(
            data.pos_at_time(TimePoint(0.225), uav).unwrap(),
            Vec3::new(2.25, 2.25, 2.25)
        );
        assert_approx_eq!(
            data.pos_at_time(TimePoint(0.3), uav).unwrap(),
            Vec3::new(3.0, 3.0, 3.0)
        );

        assert_approx_eq!(
            data.pos_at_time(TimePoint(0.4), uav).unwrap(),
            Vec3::new(-50.0, -8.0, 2.0)
        );
        assert_approx_eq!(
            data.pos_at_time(TimePoint(0.42), uav).unwrap(),
            Vec3::new(-40.0, -6.4, 1.6)
        );
        assert_approx_eq!(
            data.pos_at_time(TimePoint(0.45), uav).unwrap(),
            Vec3::new(-25.0, -4.0, 1.0)
        );
        assert_approx_eq!(
            data.pos_at_time(TimePoint(0.48), uav).unwrap(),
            Vec3::new(-10.0, -1.6, 0.4)
        );
        assert_approx_eq!(
            data.pos_at_time(TimePoint(0.5), uav).unwrap(),
            Vec3::new(0.0, 0.0, 0.0)
        );
        assert_approx_eq!(
            data.pos_at_time(TimePoint(0.5), uav).unwrap(),
            Vec3::new(0.0, 0.0, 0.0)
        );

        assert_approx_eq!(
            data.pos_at_time(TimePoint(0.51), uav).unwrap(),
            Vec3::new(0.0, 0.0, 0.0)
        );

        assert_approx_eq!(
            data.pos_at_time(TimePoint(0.6), uav).unwrap(),
            Vec3::new(0.0, 0.0, 0.0)
        );

        assert_approx_eq!(
            data.pos_at_time(TimePoint(1.0), uav).unwrap(),
            Vec3::new(0.0, 0.0, 0.0)
        );

        assert_approx_eq!(
            data.pos_at_time(TimePoint(10.5), uav).unwrap(),
            Vec3::new(0.0, 0.0, 0.0)
        );

        assert_approx_eq!(
            data.pos_at_time(TimePoint(25.7), uav).unwrap(),
            Vec3::new(0.0, 0.0, 0.0)
        );

        assert_approx_eq!(
            data.pos_at_time(TimePoint(67.0), uav).unwrap(),
            Vec3::new(0.0, 0.0, 0.0)
        );
    }
}
