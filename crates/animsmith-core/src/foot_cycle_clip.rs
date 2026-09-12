//! Pure format-neutral foot-cycle clip candidates.
//!
//! This module applies one already-planned source-to-output time map to a
//! cloned [`Clip`]. It neither proves that the supplied plan belongs to the
//! loaded source nor serializes or publishes the candidate; those are later
//! frontend transaction boundaries.

use std::collections::BTreeSet;
use std::mem::size_of;

use glam::{Quat, Vec3};

use crate::{
    Clip, ContactTimeWarpControlPointV1, ContactTransformOperationV1, DocumentShapeError,
    FootCycleMemberPlanV1, Interpolation, Track, TrackSample, TrackValues, sample_track,
};

/// Maximum tracks accepted by one V1 candidate operation.
pub const FOOT_CYCLE_CLIP_V1_MAX_TRACKS: usize = 4_096;
/// Maximum aggregate authored keyframes accepted by one V1 candidate.
pub const FOOT_CYCLE_CLIP_V1_MAX_INPUT_KEYS: usize = 1_048_576;
/// Maximum aggregate authored stored values, including cubic tangents.
///
/// This is the derived `3 * input_keys` shape maximum. A structurally valid
/// track cannot exceed it without first exceeding the input-key bound.
pub const FOOT_CYCLE_CLIP_V1_MAX_INPUT_VALUES: usize = 3 * FOOT_CYCLE_CLIP_V1_MAX_INPUT_KEYS;
/// Maximum aggregate keyframes in one V1 candidate.
pub const FOOT_CYCLE_CLIP_V1_MAX_GENERATED_KEYS: usize = 1_048_576;
/// Maximum aggregate bounded inspection work before candidate allocation.
///
/// Work counts every authored key, every map-knot probe for a linear track,
/// and every planned candidate key.
pub const FOOT_CYCLE_CLIP_V1_MAX_WORK: usize = 8_388_608;
/// Maximum UTF-8 bytes retained for a V1 candidate clip name.
pub const FOOT_CYCLE_CLIP_V1_MAX_NAME_BYTES: usize = 65_536;
/// Maximum exact V1 candidate storage payload bytes.
///
/// This is the derived ceiling for retained clip names, track rows, output key
/// times, and output values. It is not an independent admission limit: the
/// name, track, generated-key, and input-value caps already bound every term.
/// Hosts may use it to size or cap aggregate candidate retention.
pub const FOOT_CYCLE_CLIP_V1_MAX_CANDIDATE_BYTES: usize = FOOT_CYCLE_CLIP_V1_MAX_NAME_BYTES
    + FOOT_CYCLE_CLIP_V1_MAX_TRACKS * size_of::<Track>()
    + FOOT_CYCLE_CLIP_V1_MAX_GENERATED_KEYS * size_of::<f32>()
    + FOOT_CYCLE_CLIP_V1_MAX_INPUT_VALUES * size_of::<Quat>();

const _: () = assert!(FOOT_CYCLE_CLIP_V1_MAX_INPUT_VALUES == 3 * FOOT_CYCLE_CLIP_V1_MAX_INPUT_KEYS);
const _: () = assert!(FOOT_CYCLE_CLIP_V1_MAX_CANDIDATE_BYTES < usize::MAX);

/// One bounded resource owned by the V1 clip candidate operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FootCycleClipResourceV1 {
    /// Tracks inspected.
    Tracks,
    /// Authored keyframes inspected.
    InputKeys,
    /// Authored stored values inspected.
    InputValues,
    /// Candidate keyframes planned.
    GeneratedKeys,
    /// Aggregate bounded row work planned.
    Work,
    /// UTF-8 bytes retained for the candidate clip name.
    NameBytes,
}

/// Why a cubic-spline track cannot be represented by this conservative seam.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FootCycleCubicSplineRefusalV1 {
    /// Multi-key stored values differ exactly.
    DifferingValues,
    /// At least one stored input or output tangent is not exactly zero.
    NonZeroTangent,
}

/// A clip could not be represented as a deterministic V1 time-warp candidate.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum FootCycleClipWarpError {
    /// The member plan did not carry a time-warp operation.
    #[error("foot-cycle clip candidate requires a time_warp operation")]
    UnsupportedOperation,
    /// The time-warp version is not V1.
    #[error("unsupported foot-cycle time-warp version {version}")]
    UnsupportedVersion {
        /// Declared version.
        version: u32,
    },
    /// The clip duration does not narrow to a finite positive binary32 duration.
    #[error("clip duration {duration_s} does not narrow to a finite positive binary32 duration")]
    InvalidClipDuration {
        /// Declared clip duration.
        duration_s: f64,
    },
    /// The time-warp output duration differs exactly from the clip duration.
    #[error(
        "time-warp output duration {operation_duration_s} does not equal clip duration {clip_duration_s}"
    )]
    DurationMismatch {
        /// Clip duration.
        clip_duration_s: f64,
        /// Operation output duration.
        operation_duration_s: f64,
    },
    /// The control-point count is outside the closed V1 bound.
    #[error("time-warp declares {found} control points; expected 2..={maximum}")]
    InvalidControlPointCount {
        /// Observed count.
        found: usize,
        /// Maximum count.
        maximum: usize,
    },
    /// One normalized control point is non-finite or outside `[0, 1]`.
    #[error("time-warp control point {index} is not finite and normalized")]
    InvalidControlPoint {
        /// Zero-based control-point index.
        index: usize,
    },
    /// The map does not contain the exact normalized endpoints.
    #[error("time-warp must map exact endpoints (0,0) and (1,1)")]
    InvalidMapEndpoints,
    /// The source or output coordinates are not strictly increasing.
    #[error("time-warp is not strictly increasing at control point {index}")]
    NonMonotoneMap {
        /// Index of the second point in the invalid pair.
        index: usize,
    },
    /// A track violates the public structural track contract.
    #[error("track {track_index} is malformed: {source}")]
    InvalidTrack {
        /// Track index in source order.
        track_index: usize,
        /// Existing typed shape failure.
        source: DocumentShapeError,
    },
    /// Two tracks target the same property of the same bone.
    #[error("track {track_index} duplicates {property:?} for node {bone}")]
    DuplicateTrackTarget {
        /// Second track index in source order.
        track_index: usize,
        /// Duplicated bone index.
        bone: usize,
        /// Duplicated property.
        property: crate::Property,
    },
    /// One authored key lies outside the clip interval.
    #[error("track {track_index} key {key_index} at {time_s} is outside [0, {duration_s}]")]
    TrackTimeOutOfRange {
        /// Track index in source order.
        track_index: usize,
        /// Key index in source order.
        key_index: usize,
        /// Authored key time.
        time_s: f32,
        /// Narrowed clip duration.
        duration_s: f32,
    },
    /// A multi-key cubic spline is not representation-exact under this seam.
    #[error("track {track_index} cubic spline is not safely constant: {reason:?}")]
    UnsupportedCubicSpline {
        /// Track index in source order.
        track_index: usize,
        /// Exact refusal class.
        reason: FootCycleCubicSplineRefusalV1,
    },
    /// A stored quaternion key cannot be normalized by runtime sampling.
    #[error("track {track_index} quaternion key {key_index} has invalid binary32 magnitude")]
    InvalidQuaternionKey {
        /// Track index in source order.
        track_index: usize,
        /// Key index in source order.
        key_index: usize,
    },
    /// Two distinct source instants narrowed to one candidate time.
    #[error("track {track_index} generated a binary32 time collision")]
    TimeCollision {
        /// Track index in source order.
        track_index: usize,
    },
    /// Two distinct generated source instants narrowed to one binary32 time.
    #[error("track {track_index} generated a binary32 source-time collision")]
    SourceTimeCollision {
        /// Track index in source order.
        track_index: usize,
    },
    /// A fixed aggregate V1 resource limit was exceeded.
    #[error("foot-cycle clip {resource:?} count {observed} exceeds V1 limit {maximum}")]
    LimitExceeded {
        /// Bounded resource.
        resource: FootCycleClipResourceV1,
        /// Observed count, including the terminal N+1 where applicable.
        observed: usize,
        /// Fixed V1 maximum.
        maximum: usize,
    },
    /// Checked aggregate arithmetic overflowed before allocation.
    #[error("foot-cycle clip {resource:?} count overflowed")]
    CountOverflow {
        /// Resource whose checked arithmetic overflowed.
        resource: FootCycleClipResourceV1,
    },
}

/// Exact storage counts and the bounded work charge for one validated V1 clip
/// candidate.
///
/// Hosts may sum these counts with checked arithmetic before retaining a batch
/// of candidates. The counts describe the candidate that
/// [`time_warp_clip_v1`] would build from the same inputs; obtaining them does
/// not allocate or mutate that candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct FootCycleClipPreflightV1 {
    tracks: usize,
    input_keys: usize,
    input_values: usize,
    candidate_keys: usize,
    candidate_values: usize,
    name_bytes: usize,
    candidate_bytes: usize,
    work: usize,
}

impl FootCycleClipPreflightV1 {
    /// Number of tracks inspected and retained by the candidate.
    pub const fn tracks(self) -> usize {
        self.tracks
    }

    /// Number of authored keyframes inspected.
    pub const fn input_keys(self) -> usize {
        self.input_keys
    }

    /// Number of authored stored values inspected, including cubic tangents.
    pub const fn input_values(self) -> usize {
        self.input_values
    }

    /// Number of keyframes retained by the candidate.
    pub const fn candidate_keys(self) -> usize {
        self.candidate_keys
    }

    /// Number of stored values retained by the candidate.
    ///
    /// Constant cubic tracks retain their authored values verbatim. LINEAR
    /// and STEP tracks retain exactly one value per candidate key.
    pub const fn candidate_values(self) -> usize {
        self.candidate_values
    }

    /// UTF-8 bytes retained for the candidate clip name.
    pub const fn name_bytes(self) -> usize {
        self.name_bytes
    }

    /// Exact V1 candidate storage payload bytes.
    ///
    /// This is the sum of retained name bytes, track rows, output key times,
    /// and output values; it is not an allocator-reserved-capacity estimate.
    pub const fn candidate_bytes(self) -> usize {
        self.candidate_bytes
    }

    /// Conservative V1 inspection and candidate-planning work charge.
    pub const fn work(self) -> usize {
        self.work
    }
}

#[derive(Debug)]
struct PreparedClipWarp<'a> {
    output_duration_s: f64,
    points: &'a [ContactTimeWarpControlPointV1],
    duration: f32,
    identity: bool,
    counts: FootCycleClipPreflightV1,
    /// Exact retained-key capacity for each source-order track. This is
    /// bounded by the public track/key limits and is metadata, not a
    /// candidate buffer.
    track_candidate_keys: Vec<usize>,
}

/// One key the candidate clip will store, in the emitted binary32 domain.
#[derive(Debug, Clone, Copy)]
struct CandidateKey {
    output_time: f32,
    value: CandidateValue,
}

/// Where an emitted key's stored value comes from.
#[derive(Debug, Clone, Copy)]
enum CandidateValue {
    /// The authored key at this index, copied verbatim.
    Authored(usize),
    /// The track sampled at this instant, for a key no authored key is.
    Sampled(f32),
}

/// One interior V1 time-warp control point, resolved into the binary32 time
/// domain a candidate clip stores.
///
/// A candidate clip stores binary32 key times, so whether a control point is
/// the same instant as an authored key is decided in that domain and nowhere
/// else: the control point's source instant is narrowed once into
/// [`Self::source_time`], and it coincides with an authored key exactly when
/// [`Self::coincides_with`] holds for that key's stored time.
///
/// A coincident control point and authored key are one instant, so they are
/// emitted as one key: at the authored key's stored time, with the authored
/// key's stored value, and at this knot's [`Self::output_time`] — the control
/// point is the definition of the map at that instant, so the output time is
/// the knot's rather than the authored key's mapped one. A control point that
/// coincides with no authored key adds one key sampled at
/// [`Self::source_time`]. Two emitted keys that share one binary32 source
/// time, or whose output times do not strictly increase, are genuinely
/// distinct instants that collapsed, and refuse.
///
/// [`time_warp_rows_v1`] resolves the knots one LINEAR track emits and merges
/// them with its authored keys. An independent proof of a candidate clip asks
/// this one question rather than spelling coincidence a second way, but it
/// derives every stored number itself: the two times here are this resolution,
/// not an authority, so a proof narrows them again from the control point
/// [`Self::control_point_index`] names. A mistake in one resolution is then a
/// mismatch rather than a shared assumption.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FootCycleClipWarpKnotV1 {
    control_point_index: usize,
    source_time: f32,
    output_time: f32,
}

impl FootCycleClipWarpKnotV1 {
    /// Index of the control point this knot resolves, in the operation's own
    /// map.
    ///
    /// This is what a consumer that must not trust the resolution reads: both
    /// times above are `input_time`/`output_time` of that point multiplied by
    /// the narrowed binary32 duration and narrowed once, and a proof of a
    /// candidate does that arithmetic itself.
    #[must_use]
    pub const fn control_point_index(self) -> usize {
        self.control_point_index
    }

    /// The control point's source instant, narrowed once to the stored domain.
    #[must_use]
    pub const fn source_time(self) -> f32 {
        self.source_time
    }

    /// The output time the emitted key carries.
    #[must_use]
    pub const fn output_time(self) -> f32 {
        self.output_time
    }

    /// Whether the binary32 `time` is this same instant.
    ///
    /// `time` is an authored key's stored time, or another knot's
    /// [`Self::source_time`] when the question is whether a plan names two
    /// breakpoints the emitted domain cannot tell apart. A control point and
    /// an authored key are two computations of one instant: the plan carries a
    /// normalized phase, the track carries a binary32 time, and reconstructing
    /// the phase as `input_time * duration` reproduces that stored time or
    /// lands one representable place beside it, on either side.
    /// Binary32 is the domain the candidate stores and nothing is
    /// representable between two adjacent values there, so emitting a key at
    /// each would spell one instant twice. The predicate therefore admits both
    /// and nothing wider: a key two places away is a different instant. It
    /// asks only about a reconstructed instant; two authored keys one place
    /// apart remain the two instants the track authored.
    #[must_use]
    pub fn coincides_with(self, time: f32) -> bool {
        self.source_time == time
            || self.source_time.next_up() == time
            || time.next_up() == self.source_time
    }
}

/// Resolve the knots one `track` emits for a validated V1 time warp.
///
/// [`time_warp_rows_v1`] is the public form of this: it merges these knots
/// with the track's authored keys into the sequence a candidate stores.
///
/// `control_points` is the operation's validated strictly increasing map and
/// `duration` the clip's narrowed binary32 duration. Only a LINEAR track emits
/// knots: a STEP track maps its authored breakpoints and a retained cubic
/// track is copied. The map's endpoints are exact, so only its interior points
/// can contribute a knot, and only where the narrowed instant is the same
/// instant as one of the track's own first and last authored keys, or falls
/// strictly between them. The span test is inclusive on purpose: a knot one
/// representable place outside the span is the same instant as the key at that
/// end, and which side of a rounding step it lands on must not decide whether
/// that key takes its control point's output.
///
/// A knot that is the same instant as a source endpoint — `0` or `duration` —
/// contributes nothing at all. The map evaluates its exact `(0,0)` and
/// `(1,1)` rows there, so a candidate retains those two instants as
/// themselves; letting an interior point re-time a key there would silently
/// start the candidate late or end it early instead. That is the only
/// exception: every other authored key the map has a control point for takes
/// that point's output.
///
/// Knots are yielded in nondecreasing source order.
///
/// The operation must already be validated, as [`time_warp_clip_v1`] does
/// before reaching here: a control point that is not finite and normalized
/// narrows to a non-finite instant, which fails every comparison above and is
/// dropped here rather than refused. This resolver reports no error of its
/// own.
fn time_warp_knots_v1<'a>(
    control_points: &'a [ContactTimeWarpControlPointV1],
    duration: f32,
    track: &Track,
) -> impl Iterator<Item = FootCycleClipWarpKnotV1> + 'a {
    let start_time = track.start_time();
    let end_time = track.end_time();
    let interior = if track.interpolation == Interpolation::Linear {
        control_points
            .get(1..control_points.len().saturating_sub(1))
            .unwrap_or_default()
    } else {
        &[]
    };
    interior
        .iter()
        .enumerate()
        .map(move |(offset, point)| FootCycleClipWarpKnotV1 {
            // The interior slice starts at the map's second row.
            control_point_index: offset + 1,
            // Validated normalized control-point coordinates and the finite
            // binary32 duration keep both products finite through narrowing.
            source_time: (point.input_time() * f64::from(duration)) as f32,
            output_time: (point.output_time() * f64::from(duration)) as f32,
        })
        .filter(move |knot| {
            (knot.coincides_with(start_time)
                || knot.coincides_with(end_time)
                || (knot.source_time > start_time && knot.source_time < end_time))
                && !knot.coincides_with(0.0)
                && !knot.coincides_with(duration)
        })
}

/// Apply one member's validated normalized source-to-output map to a cloned
/// format-neutral clip candidate.
///
/// The function does not verify that `plan.input()` names `input`; the format
/// frontend must bind the selected source to the plan before calling this pure
/// candidate builder. It validates only track-local [`Clip`] shape: because
/// this seam does not receive a skeleton, the host must already have bound and
/// validated every track bone index against its selected skeleton. On every
/// error the borrowed clip is unchanged.
///
/// # Errors
///
/// Returns [`FootCycleClipWarpError`] for an unsupported or malformed plan,
/// invalid clip/track shape, unsafe cubic spline, binary32 time collision, or
/// exceeded fixed work/resource bound.
pub fn time_warp_clip_v1(
    input: &Clip,
    plan: &FootCycleMemberPlanV1,
) -> Result<Clip, FootCycleClipWarpError> {
    let prepared = prepare_clip_warp(input, plan)?;
    debug_assert!(prepared.counts.tracks <= FOOT_CYCLE_CLIP_V1_MAX_TRACKS);

    if prepared.identity {
        return Ok(input.clone());
    }

    let mut tracks = Vec::with_capacity(input.tracks.len());
    debug_assert_eq!(prepared.track_candidate_keys.len(), input.tracks.len());
    for (track_index, track) in input.tracks.iter().enumerate() {
        let candidate_keys = prepared.track_candidate_keys[track_index];
        tracks.push(warp_track(
            track,
            track_index,
            prepared.points,
            prepared.duration,
            candidate_keys,
        )?);
    }
    Ok(Clip {
        name: input.name.clone(),
        duration_s: prepared.output_duration_s,
        tracks,
    })
}

/// Validate and count one V1 clip candidate without allocating it.
///
/// This is the authoritative pre-allocation boundary used by
/// [`time_warp_clip_v1`]. Hosts that retain multiple candidates can sum the
/// returned counts with checked arithmetic and enforce an invocation-level
/// budget before constructing any candidate. Like the builder, this Clip-only
/// boundary validates track-local shape; the host owns skeleton binding and
/// track-bone-index validation.
///
/// # Errors
///
/// Returns the same validation, shape, interpolation, and fixed-limit errors
/// that candidate construction can report before allocating the output clip.
pub fn preflight_time_warp_clip_v1(
    input: &Clip,
    plan: &FootCycleMemberPlanV1,
) -> Result<FootCycleClipPreflightV1, FootCycleClipWarpError> {
    Ok(prepare_clip_warp(input, plan)?.counts)
}

fn prepare_clip_warp<'a>(
    input: &Clip,
    plan: &'a FootCycleMemberPlanV1,
) -> Result<PreparedClipWarp<'a>, FootCycleClipWarpError> {
    let (output_duration_s, points) = validate_operation(input, plan.operation())?;
    let duration = input.duration_s as f32;
    let identity = points
        .iter()
        .all(|point| point.input_time() == point.output_time());
    let preflight = preflight(input, points, duration, identity)?;
    Ok(PreparedClipWarp {
        output_duration_s,
        points,
        duration,
        identity,
        counts: preflight.counts,
        track_candidate_keys: preflight.track_candidate_keys,
    })
}

fn validate_operation<'a>(
    input: &Clip,
    operation: &'a ContactTransformOperationV1,
) -> Result<(f64, &'a [ContactTimeWarpControlPointV1]), FootCycleClipWarpError> {
    let duration = input.duration_s as f32;
    if !input.duration_s.is_finite()
        || input.duration_s <= 0.0
        || !duration.is_finite()
        || duration <= 0.0
    {
        return Err(FootCycleClipWarpError::InvalidClipDuration {
            duration_s: input.duration_s,
        });
    }
    let ContactTransformOperationV1::TimeWarp {
        version,
        output_duration_s,
        control_points,
    } = operation
    else {
        return Err(FootCycleClipWarpError::UnsupportedOperation);
    };
    if *version != 1 {
        return Err(FootCycleClipWarpError::UnsupportedVersion { version: *version });
    }
    if *output_duration_s != input.duration_s {
        return Err(FootCycleClipWarpError::DurationMismatch {
            clip_duration_s: input.duration_s,
            operation_duration_s: *output_duration_s,
        });
    }
    if !(2..=crate::CONTACT_TRANSFORM_RESULT_V1_MAX_CONTROL_POINTS).contains(&control_points.len())
    {
        return Err(FootCycleClipWarpError::InvalidControlPointCount {
            found: control_points.len(),
            maximum: crate::CONTACT_TRANSFORM_RESULT_V1_MAX_CONTROL_POINTS,
        });
    }
    for (index, point) in control_points.iter().enumerate() {
        if !point.input_time().is_finite()
            || !point.output_time().is_finite()
            || !(0.0..=1.0).contains(&point.input_time())
            || !(0.0..=1.0).contains(&point.output_time())
        {
            return Err(FootCycleClipWarpError::InvalidControlPoint { index });
        }
    }
    if control_points
        .first()
        .is_none_or(|point| point.input_time() != 0.0 || point.output_time() != 0.0)
        || control_points
            .last()
            .is_none_or(|point| point.input_time() != 1.0 || point.output_time() != 1.0)
    {
        return Err(FootCycleClipWarpError::InvalidMapEndpoints);
    }
    for (index, pair) in control_points.windows(2).enumerate() {
        if pair[0].input_time() >= pair[1].input_time()
            || pair[0].output_time() >= pair[1].output_time()
        {
            return Err(FootCycleClipWarpError::NonMonotoneMap { index: index + 1 });
        }
    }
    Ok((*output_duration_s, control_points))
}

#[derive(Debug)]
struct PreflightedClipWarp {
    counts: FootCycleClipPreflightV1,
    track_candidate_keys: Vec<usize>,
}

fn preflight(
    input: &Clip,
    points: &[ContactTimeWarpControlPointV1],
    duration: f32,
    identity: bool,
) -> Result<PreflightedClipWarp, FootCycleClipWarpError> {
    let mut counts = FootCycleClipPreflightV1 {
        tracks: input.tracks.len(),
        input_keys: 0,
        input_values: 0,
        candidate_keys: 0,
        candidate_values: 0,
        name_bytes: input.name.len(),
        candidate_bytes: 0,
        work: 0,
    };
    check_limit(
        FootCycleClipResourceV1::Tracks,
        counts.tracks,
        FOOT_CYCLE_CLIP_V1_MAX_TRACKS,
    )?;
    check_limit(
        FootCycleClipResourceV1::NameBytes,
        counts.name_bytes,
        FOOT_CYCLE_CLIP_V1_MAX_NAME_BYTES,
    )?;
    counts.candidate_bytes = counts.name_bytes + counts.tracks * size_of::<Track>();
    let mut targets = BTreeSet::new();
    let mut track_candidate_keys = Vec::with_capacity(input.tracks.len());
    for (track_index, track) in input.tracks.iter().enumerate() {
        if !targets.insert((track.bone, track.property)) {
            return Err(FootCycleClipWarpError::DuplicateTrackTarget {
                track_index,
                bone: track.bone,
                property: track.property,
            });
        }
        counts.input_keys = checked_add(
            FootCycleClipResourceV1::InputKeys,
            counts.input_keys,
            track.times.len(),
        )?;
        check_limit(
            FootCycleClipResourceV1::InputKeys,
            counts.input_keys,
            FOOT_CYCLE_CLIP_V1_MAX_INPUT_KEYS,
        )?;
        counts.input_values = checked_add(
            FootCycleClipResourceV1::InputValues,
            counts.input_values,
            track.values.len(),
        )?;
        check_limit(
            FootCycleClipResourceV1::InputValues,
            counts.input_values,
            FOOT_CYCLE_CLIP_V1_MAX_INPUT_VALUES,
        )?;
        crate::model::validate_track_shape(0, track).map_err(|source| {
            FootCycleClipWarpError::InvalidTrack {
                track_index,
                source,
            }
        })?;
        for (key_index, &time) in track.times.iter().enumerate() {
            if time < 0.0 || time > duration {
                return Err(FootCycleClipWarpError::TrackTimeOutOfRange {
                    track_index,
                    key_index,
                    time_s: time,
                    duration_s: duration,
                });
            }
        }
        if let TrackValues::Quats(values) = &track.values {
            for key_index in 0..track.times.len() {
                let value = values[track.value_index(key_index)];
                if !value.length_squared().is_finite() || value.length_squared() <= 0.0 {
                    return Err(FootCycleClipWarpError::InvalidQuaternionKey {
                        track_index,
                        key_index,
                    });
                }
            }
        }
        if track.interpolation == Interpolation::CubicSpline {
            validate_cubic(track, track_index)?;
        }
        let mut generated = 0;
        if !identity && track.interpolation != Interpolation::CubicSpline {
            visit_warp_track_keys(track, track_index, points, duration, |_key| {
                generated = checked_add(FootCycleClipResourceV1::GeneratedKeys, generated, 1)?;
                Ok(())
            })?;
        } else {
            generated = track.times.len();
        }
        track_candidate_keys.push(generated);
        counts.candidate_keys = checked_add(
            FootCycleClipResourceV1::GeneratedKeys,
            counts.candidate_keys,
            generated,
        )?;
        check_limit(
            FootCycleClipResourceV1::GeneratedKeys,
            counts.candidate_keys,
            FOOT_CYCLE_CLIP_V1_MAX_GENERATED_KEYS,
        )?;
        // Candidate value storage has no independent limit to check here.
        // Constant cubic tracks are retained verbatim, so InputValues bounds
        // them. LINEAR and STEP emit exactly one value per generated key, so
        // GeneratedKeys bounds them. A separate generated-value refusal would
        // therefore be unreachable and would only duplicate those authorities.
        let candidate_values = if track.interpolation == Interpolation::CubicSpline {
            track.values.len()
        } else {
            generated
        };
        let governing_resource = if track.interpolation == Interpolation::CubicSpline {
            FootCycleClipResourceV1::InputValues
        } else {
            FootCycleClipResourceV1::GeneratedKeys
        };
        counts.candidate_values = checked_add(
            governing_resource,
            counts.candidate_values,
            candidate_values,
        )?;
        // The admitted name/track/key/value caps make every storage term fit
        // in usize and sum to at most MAX_CANDIDATE_BYTES. This exact count is
        // host-facing accounting, not another independently-refusable limit.
        counts.candidate_bytes += candidate_track_bytes(track, generated, candidate_values);
        counts.work = checked_add(
            FootCycleClipResourceV1::Work,
            counts.work,
            track.times.len(),
        )?;
        if !identity && track.interpolation == Interpolation::Linear {
            counts.work = checked_add(FootCycleClipResourceV1::Work, counts.work, points.len())?;
        }
        counts.work = checked_add(FootCycleClipResourceV1::Work, counts.work, generated)?;
        check_limit(
            FootCycleClipResourceV1::Work,
            counts.work,
            FOOT_CYCLE_CLIP_V1_MAX_WORK,
        )?;
    }
    Ok(PreflightedClipWarp {
        counts,
        track_candidate_keys,
    })
}

/// One row of the key sequence a track emits for a validated V1 time warp.
///
/// The rows say which keys a candidate stores and in what order. A knot-
/// bearing row carries the [`FootCycleClipWarpKnotV1`] it names — its
/// control-point index and that point's two narrowed times — and no stored
/// value. A consumer that must not trust the producer reads the index and
/// narrows the control point itself, as the independent `ClipMap` proof in
/// the `animsmith` crate does, so a builder and that proof share one answer to
/// [`FootCycleClipWarpKnotV1::coincides_with`] — the question that had three
/// different answers before, and the only one whose disagreement is silent —
/// while every number each of them stores stays its own.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FootCycleClipWarpRowV1 {
    /// The authored key at this index, at its own time, mapped by the warp.
    Authored(usize),
    /// A knot that is no authored key: one key sampled at the instant its
    /// control point resolves to.
    Knot(FootCycleClipWarpKnotV1),
    /// One authored key and the knot that is the same instant, as one key: the
    /// authored time and value, at the instant this knot's control point maps
    /// that to.
    Coalesced(usize, FootCycleClipWarpKnotV1),
}

/// Merge a track's authored keys with its resolved knots into the key sequence
/// the candidate emits, in source order.
///
/// A knot is one interior control point of the validated map, resolved once
/// into the binary32 time domain the candidate stores; see
/// [`FootCycleClipWarpKnotV1`]. Only a LINEAR track has knots, and only where
/// the narrowed instant is the same instant as the track's first or last
/// authored key — one representable place on either side of it counts — or
/// falls strictly between them. A knot that is the same instant as a source
/// endpoint, `0` or the duration, is the one exception and contributes
/// nothing: the map's exact `(0,0)` and `(1,1)` rows define the output there.
///
/// A knot that is the same instant as the next authored key coalesces with it;
/// otherwise whichever comes first in the emitted binary32 domain is yielded
/// first. Authored key times strictly increase and knots arrive in
/// nondecreasing source order, so the merge is one pass and the rows name each
/// authored index at most once.
///
/// Binding is total even where two authored keys are one place apart. A key
/// the knot's source time equals wins over a key it is merely beside, so a
/// knot that is bit-equal to the later of such a pair binds to that later key
/// and the earlier one keeps its own mapped output. Two keys exactly two
/// places apart can both be beside one knot without either being equal to it;
/// the earlier binds, and the later stays its own instant. Either way both
/// authored keys are retained: coalescing re-times an authored key, it never
/// removes one.
///
/// The rows are not themselves a refusal check: two knots that name one
/// instant — directly, or through the same authored key, which only one of
/// them can coalesce into — still arrive as two rows, and the producer refuses
/// them. A consumer proving a candidate sees the same two rows and never
/// reaches that candidate.
pub fn time_warp_rows_v1<'a>(
    control_points: &'a [ContactTimeWarpControlPointV1],
    duration: f32,
    track: &'a Track,
) -> impl Iterator<Item = FootCycleClipWarpRowV1> + 'a {
    WarpRows {
        times: &track.times,
        authored_index: 0,
        knots: time_warp_knots_v1(control_points, duration, track).peekable(),
    }
}

struct WarpRows<'a, K: Iterator<Item = FootCycleClipWarpKnotV1>> {
    times: &'a [f32],
    authored_index: usize,
    knots: std::iter::Peekable<K>,
}

impl<K: Iterator<Item = FootCycleClipWarpKnotV1>> WarpRows<'_, K> {
    fn next_authored(&mut self) -> Option<FootCycleClipWarpRowV1> {
        let index = self.authored_index;
        self.times.get(index)?;
        self.authored_index += 1;
        Some(FootCycleClipWarpRowV1::Authored(index))
    }
}

impl<K: Iterator<Item = FootCycleClipWarpKnotV1>> Iterator for WarpRows<'_, K> {
    type Item = FootCycleClipWarpRowV1;

    fn next(&mut self) -> Option<Self::Item> {
        let Some(knot) = self.knots.peek().copied() else {
            return self.next_authored();
        };
        if let Some(&authored_time) = self.times.get(self.authored_index) {
            // A key the knot is exactly is the key it binds to, so yield a
            // merely adjacent key first when the next one is that key.
            let equal_key_follows = self
                .times
                .get(self.authored_index + 1)
                .is_some_and(|&next| next == knot.source_time());
            if knot.coincides_with(authored_time) && !equal_key_follows {
                self.knots.next();
                let index = self.authored_index;
                self.authored_index += 1;
                return Some(FootCycleClipWarpRowV1::Coalesced(index, knot));
            }
            if authored_time < knot.source_time() {
                return self.next_authored();
            }
        }
        self.knots.next();
        Some(FootCycleClipWarpRowV1::Knot(knot))
    }
}

/// Visit every retained candidate key in source order, validating the emitted
/// binary32 source and output times before the caller can allocate or retain a
/// candidate buffer. Both preflight and the builder use this one traversal so
/// a newly added refusal cannot silently become builder-only.
///
/// [`time_warp_rows_v1`] decides which keys the track emits and in what order;
/// this attaches the builder's own output times and values to them.
///
/// Two knots that name one instant are a
/// [`FootCycleClipWarpError::SourceTimeCollision`]: the plan names two
/// breakpoints the emitted domain cannot tell apart. They name one instant
/// directly when their source times are equal or adjacent, and through an
/// authored key when both are the same instant as it — only one can coalesce
/// into it, and the other would publish as its own key one place from it. Each
/// knot is therefore compared against both instants of the knot before it: its
/// own resolved source time, and the instant it named, which is the authored
/// key's time when it coalesced. Neither alone is enough, because a knot that
/// coalesced answers to a time one place from its own. Two authored keys one
/// place apart are not this case: they are the instants the track authored,
/// and both are retained.
fn visit_warp_track_keys(
    track: &Track,
    track_index: usize,
    points: &[ContactTimeWarpControlPointV1],
    duration: f32,
    mut visit: impl FnMut(CandidateKey) -> Result<(), FootCycleClipWarpError>,
) -> Result<(), FootCycleClipWarpError> {
    let mut previous = None;
    let mut previous_knot: Option<(FootCycleClipWarpKnotV1, f32)> = None;
    for row in time_warp_rows_v1(points, duration, track) {
        let named = match row {
            FootCycleClipWarpRowV1::Authored(_) => None,
            FootCycleClipWarpRowV1::Knot(knot) => Some((knot, knot.source_time())),
            FootCycleClipWarpRowV1::Coalesced(index, knot) => Some((knot, track.times[index])),
        };
        if let (Some((previous, named_instant)), Some((knot, _))) = (previous_knot, named)
            && (knot.coincides_with(previous.source_time()) || knot.coincides_with(named_instant))
        {
            return Err(FootCycleClipWarpError::SourceTimeCollision { track_index });
        }
        if named.is_some() {
            previous_knot = named;
        }
        let key = match row {
            FootCycleClipWarpRowV1::Authored(index) => CandidateKey {
                output_time: map_time(track.times[index], duration, points),
                value: CandidateValue::Authored(index),
            },
            FootCycleClipWarpRowV1::Knot(knot) => CandidateKey {
                output_time: knot.output_time(),
                value: CandidateValue::Sampled(knot.source_time()),
            },
            FootCycleClipWarpRowV1::Coalesced(index, knot) => CandidateKey {
                output_time: knot.output_time(),
                value: CandidateValue::Authored(index),
            },
        };
        validate_candidate_key(previous, key, track_index)?;
        previous = Some(key);
        visit(key)?;
    }
    Ok(())
}

fn validate_cubic(track: &Track, track_index: usize) -> Result<(), FootCycleClipWarpError> {
    if track.times.len() <= 1 {
        return Ok(());
    }
    match &track.values {
        TrackValues::Vec3s(values) => {
            let reference = values[1];
            for key in 0..track.times.len() {
                if !same_vec3(values[3 * key + 1], reference) {
                    return Err(FootCycleClipWarpError::UnsupportedCubicSpline {
                        track_index,
                        reason: FootCycleCubicSplineRefusalV1::DifferingValues,
                    });
                }
                if values[3 * key] != Vec3::ZERO || values[3 * key + 2] != Vec3::ZERO {
                    return Err(FootCycleClipWarpError::UnsupportedCubicSpline {
                        track_index,
                        reason: FootCycleCubicSplineRefusalV1::NonZeroTangent,
                    });
                }
            }
        }
        TrackValues::Quats(values) => {
            let reference = values[1];
            for key in 0..track.times.len() {
                if !same_quat(values[3 * key + 1], reference) {
                    return Err(FootCycleClipWarpError::UnsupportedCubicSpline {
                        track_index,
                        reason: FootCycleCubicSplineRefusalV1::DifferingValues,
                    });
                }
                if values[3 * key] != Quat::from_xyzw(0.0, 0.0, 0.0, 0.0)
                    || values[3 * key + 2] != Quat::from_xyzw(0.0, 0.0, 0.0, 0.0)
                {
                    return Err(FootCycleClipWarpError::UnsupportedCubicSpline {
                        track_index,
                        reason: FootCycleCubicSplineRefusalV1::NonZeroTangent,
                    });
                }
            }
        }
    }
    Ok(())
}

fn same_vec3(left: Vec3, right: Vec3) -> bool {
    left.to_array()
        .into_iter()
        .zip(right.to_array())
        .all(|(left, right)| left.to_bits() == right.to_bits())
}

fn same_quat(left: Quat, right: Quat) -> bool {
    left.to_array()
        .into_iter()
        .zip(right.to_array())
        .all(|(left, right)| left.to_bits() == right.to_bits())
}

/// Refuse emitted output times that do not strictly increase.
///
/// Source times are the other half of the same obligation and are refused one
/// level up, where the plan's own knots are compared to each other rather than
/// to the authored keys they may have been coalesced into.
fn validate_candidate_key(
    previous: Option<CandidateKey>,
    key: CandidateKey,
    track_index: usize,
) -> Result<(), FootCycleClipWarpError> {
    if let Some(previous) = previous
        && previous.output_time >= key.output_time
    {
        return Err(FootCycleClipWarpError::TimeCollision { track_index });
    }
    Ok(())
}

/// Return one already-validated candidate value.
///
/// LINEAR Vec3 sampling is a convex weighted sum with a source time in
/// `[0, 1]`, so finite endpoints remain finite even at binary32 extremes.
/// Quaternion keys have a finite positive squared length before this point;
/// normalizing them and slerping two finite unit quaternions also stays finite.
fn candidate_value(track: &Track, key: CandidateKey) -> TrackSample {
    match (&track.values, key.value) {
        (TrackValues::Vec3s(authored), CandidateValue::Authored(index)) => {
            TrackSample::Vec3(authored[index])
        }
        (TrackValues::Quats(authored), CandidateValue::Authored(index)) => {
            TrackSample::Quat(authored[index])
        }
        (_, CandidateValue::Sampled(source_time)) => sample_track(track, source_time),
    }
}

fn warp_track(
    track: &Track,
    track_index: usize,
    points: &[ContactTimeWarpControlPointV1],
    duration: f32,
    candidate_keys: usize,
) -> Result<Track, FootCycleClipWarpError> {
    if track.interpolation == Interpolation::CubicSpline {
        return Ok(track.clone());
    }
    let mut times = Vec::with_capacity(candidate_keys);
    let (mut vec3s, mut quats) = match &track.values {
        TrackValues::Vec3s(_) => (Vec::with_capacity(candidate_keys), Vec::new()),
        TrackValues::Quats(_) => (Vec::new(), Vec::with_capacity(candidate_keys)),
    };
    visit_warp_track_keys(track, track_index, points, duration, |key| {
        times.push(key.output_time);
        match candidate_value(track, key) {
            TrackSample::Vec3(value) => vec3s.push(value),
            TrackSample::Quat(value) => quats.push(value),
        }
        Ok(())
    })?;
    let values = match &track.values {
        TrackValues::Vec3s(_) => TrackValues::Vec3s(vec3s),
        TrackValues::Quats(_) => TrackValues::Quats(quats),
    };
    Ok(Track {
        bone: track.bone,
        property: track.property,
        interpolation: track.interpolation,
        times,
        values,
    })
}

fn map_time(source_time: f32, duration: f32, points: &[ContactTimeWarpControlPointV1]) -> f32 {
    let normalized = f64::from(source_time) / f64::from(duration);
    let upper = points.partition_point(|point| point.input_time() <= normalized);
    let right = upper.clamp(1, points.len() - 1);
    let left = right - 1;
    let x0 = points[left].input_time();
    let x1 = points[right].input_time();
    let y0 = points[left].output_time();
    let y1 = points[right].output_time();
    let fraction = (normalized - x0) / (x1 - x0);
    let mapped = (y0 + fraction * (y1 - y0)) * f64::from(duration);
    // Validated inputs keep normalized and mapped time in [0, 1]; multiplying
    // by the finite binary32 duration therefore remains finite. At source
    // endpoints this calculation evaluates the exact (0, 0) and (1, 1) map
    // rows, so their binary32 endpoints are retained without a second refusal.
    mapped as f32
}

fn checked_add(
    resource: FootCycleClipResourceV1,
    left: usize,
    right: usize,
) -> Result<usize, FootCycleClipWarpError> {
    left.checked_add(right)
        .ok_or(FootCycleClipWarpError::CountOverflow { resource })
}

fn candidate_track_bytes(track: &Track, candidate_keys: usize, candidate_values: usize) -> usize {
    candidate_keys * size_of::<f32>()
        + candidate_values
            * match &track.values {
                TrackValues::Vec3s(_) => size_of::<Vec3>(),
                TrackValues::Quats(_) => size_of::<Quat>(),
            }
}

fn check_limit(
    resource: FootCycleClipResourceV1,
    observed: usize,
    maximum: usize,
) -> Result<(), FootCycleClipWarpError> {
    if observed > maximum {
        return Err(FootCycleClipWarpError::LimitExceeded {
            resource,
            observed,
            maximum,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ContactTransformIntervalV1, Property, TrackShapeViolation,
        foot_cycle::clip_test_member_plan,
    };

    fn point(input: f64, output: f64) -> ContactTimeWarpControlPointV1 {
        ContactTimeWarpControlPointV1::new(input, output)
    }

    fn plan(duration: f64, points: &[(f64, f64)]) -> FootCycleMemberPlanV1 {
        clip_test_member_plan(ContactTransformOperationV1::time_warp(
            duration,
            points
                .iter()
                .map(|&(input, output)| point(input, output))
                .collect(),
        ))
    }

    fn dense_points(count: usize, identity: bool) -> Vec<(f64, f64)> {
        (0..count)
            .map(|index| {
                let input = index as f64 / (count - 1) as f64;
                let output = if identity || index == 0 || index + 1 == count {
                    input
                } else {
                    0.05 + 0.9 * input
                };
                (input, output)
            })
            .collect()
    }

    fn dense_vec_track(interpolation: Interpolation, count: usize) -> Track {
        let denominator = (count - 1) as f32;
        vec_track(
            interpolation,
            (0..count).map(|index| index as f32 / denominator).collect(),
            vec![Vec3::ZERO; count],
        )
    }

    fn non_identity_plan(duration: f64) -> FootCycleMemberPlanV1 {
        plan(duration, &[(0.0, 0.0), (0.25, 0.5), (1.0, 1.0)])
    }

    fn vec_track(interpolation: Interpolation, times: Vec<f32>, values: Vec<Vec3>) -> Track {
        Track {
            bone: 7,
            property: Property::Translation,
            interpolation,
            times,
            values: TrackValues::Vec3s(values),
        }
    }

    fn quat_track(interpolation: Interpolation, times: Vec<f32>, values: Vec<Quat>) -> Track {
        Track {
            bone: 9,
            property: Property::Rotation,
            interpolation,
            times,
            values: TrackValues::Quats(values),
        }
    }

    fn clip(duration_s: f64, tracks: Vec<Track>) -> Clip {
        Clip {
            name: "walk_forward".into(),
            duration_s,
            tracks,
        }
    }

    fn vec_values(track: &Track) -> &[Vec3] {
        match &track.values {
            TrackValues::Vec3s(values) => values,
            TrackValues::Quats(_) => panic!("expected Vec3 values"),
        }
    }

    fn quat_values(track: &Track) -> &[Quat] {
        match &track.values {
            TrackValues::Quats(values) => values,
            TrackValues::Vec3s(_) => panic!("expected quaternion values"),
        }
    }

    fn assert_clip_bits_equal(left: &Clip, right: &Clip) {
        assert_eq!(left.name, right.name);
        assert_eq!(left.duration_s.to_bits(), right.duration_s.to_bits());
        assert_eq!(left.tracks.len(), right.tracks.len());
        for (left, right) in left.tracks.iter().zip(&right.tracks) {
            assert_eq!(left.bone, right.bone);
            assert_eq!(left.property, right.property);
            assert_eq!(left.interpolation, right.interpolation);
            assert_eq!(
                left.times
                    .iter()
                    .map(|time| time.to_bits())
                    .collect::<Vec<_>>(),
                right
                    .times
                    .iter()
                    .map(|time| time.to_bits())
                    .collect::<Vec<_>>()
            );
            match (&left.values, &right.values) {
                (TrackValues::Vec3s(left), TrackValues::Vec3s(right)) => assert_eq!(
                    left.iter()
                        .flat_map(|value| value.to_array().map(f32::to_bits))
                        .collect::<Vec<_>>(),
                    right
                        .iter()
                        .flat_map(|value| value.to_array().map(f32::to_bits))
                        .collect::<Vec<_>>()
                ),
                (TrackValues::Quats(left), TrackValues::Quats(right)) => assert_eq!(
                    left.iter()
                        .flat_map(|value| value.to_array().map(f32::to_bits))
                        .collect::<Vec<_>>(),
                    right
                        .iter()
                        .flat_map(|value| value.to_array().map(f32::to_bits))
                        .collect::<Vec<_>>()
                ),
                _ => panic!("candidate changed track value storage kind"),
            }
        }
    }

    fn assert_approx(left: f32, right: f32) {
        assert!(
            (left - right).abs() <= 2.0 * f32::EPSILON,
            "{left} != {right}"
        );
    }

    fn assert_error(
        result: Result<Clip, FootCycleClipWarpError>,
        expected: FootCycleClipWarpError,
    ) {
        assert_eq!(result.unwrap_err(), expected);
    }

    fn assert_preflight_and_candidate_error(
        source: &Clip,
        plan: &FootCycleMemberPlanV1,
        expected: FootCycleClipWarpError,
    ) {
        assert_eq!(
            preflight_time_warp_clip_v1(source, plan),
            Err(expected.clone())
        );
        assert_error(time_warp_clip_v1(source, plan), expected);
    }

    #[test]
    fn public_preflight_reports_exact_candidate_storage_and_bounded_work() {
        let linear = vec_track(
            Interpolation::Linear,
            vec![0.0, 1.0],
            vec![Vec3::ZERO, Vec3::ONE],
        );
        let mut cubic = vec_track(
            Interpolation::CubicSpline,
            vec![0.0, 1.0],
            vec![Vec3::ZERO; 6],
        );
        cubic.bone = 8;
        let step = Track {
            bone: 9,
            property: Property::Rotation,
            interpolation: Interpolation::Step,
            times: vec![0.0, 1.0],
            values: TrackValues::Quats(vec![Quat::IDENTITY, Quat::from_rotation_y(0.5)]),
        };
        let source = clip(1.0, vec![linear, cubic, step]);
        let plan = non_identity_plan(1.0);

        let preflight = preflight_time_warp_clip_v1(&source, &plan).unwrap();
        assert_eq!(preflight.tracks(), 3);
        assert_eq!(preflight.input_keys(), 6);
        assert_eq!(preflight.input_values(), 10);
        assert_eq!(preflight.candidate_keys(), 7);
        assert_eq!(preflight.candidate_values(), 11);
        assert_eq!(preflight.name_bytes(), source.name.len());
        assert_eq!(
            preflight.candidate_bytes(),
            source.name.len()
                + 3 * size_of::<Track>()
                + 7 * size_of::<f32>()
                + 9 * size_of::<Vec3>()
                + 2 * size_of::<Quat>()
        );
        assert!(preflight.candidate_bytes() <= FOOT_CYCLE_CLIP_V1_MAX_CANDIDATE_BYTES);
        assert_eq!(preflight.work(), 16);

        let candidate = time_warp_clip_v1(&source, &plan).unwrap();
        assert_eq!(
            candidate
                .tracks
                .iter()
                .map(|track| track.times.len())
                .sum::<usize>(),
            preflight.candidate_keys()
        );
        assert_eq!(
            candidate
                .tracks
                .iter()
                .map(|track| track.values.len())
                .sum::<usize>(),
            preflight.candidate_values()
        );
    }

    #[test]
    fn public_name_storage_bound_refuses_before_identity_or_nonidentity_candidate_allocation() {
        let plans = [plan(1.0, &[(0.0, 0.0), (1.0, 1.0)]), non_identity_plan(1.0)];
        for plan in &plans {
            let mut exact = clip(
                1.0,
                vec![vec_track(Interpolation::Step, vec![0.0], vec![Vec3::ZERO])],
            );
            exact.name = "é".repeat(FOOT_CYCLE_CLIP_V1_MAX_NAME_BYTES / "é".len());
            let preflight = preflight_time_warp_clip_v1(&exact, plan).unwrap();
            assert_eq!(preflight.name_bytes(), FOOT_CYCLE_CLIP_V1_MAX_NAME_BYTES);
            assert_eq!(time_warp_clip_v1(&exact, plan).unwrap().name, exact.name);

            let mut first_excess = exact.clone();
            first_excess.name.push('é');
            let before = first_excess.clone();
            let expected = FootCycleClipWarpError::LimitExceeded {
                resource: FootCycleClipResourceV1::NameBytes,
                observed: FOOT_CYCLE_CLIP_V1_MAX_NAME_BYTES + "é".len(),
                maximum: FOOT_CYCLE_CLIP_V1_MAX_NAME_BYTES,
            };
            assert_preflight_and_candidate_error(&first_excess, plan, expected);
            assert_clip_bits_equal(&first_excess, &before);
        }
    }

    #[test]
    fn maximum_step_tracks_and_map_knots_retain_only_the_preflighted_rows() {
        let source = clip(
            1.0,
            (0..FOOT_CYCLE_CLIP_V1_MAX_TRACKS)
                .map(|bone| Track {
                    bone,
                    property: Property::Translation,
                    interpolation: Interpolation::Step,
                    times: vec![0.5],
                    values: TrackValues::Vec3s(vec![Vec3::splat(bone as f32)]),
                })
                .collect(),
        );
        let plan = plan(
            1.0,
            &dense_points(crate::CONTACT_TRANSFORM_RESULT_V1_MAX_CONTROL_POINTS, false),
        );

        let preflight = preflight_time_warp_clip_v1(&source, &plan).unwrap();
        assert_eq!(preflight.candidate_keys(), FOOT_CYCLE_CLIP_V1_MAX_TRACKS);
        assert_eq!(preflight.candidate_values(), FOOT_CYCLE_CLIP_V1_MAX_TRACKS);

        let candidate = time_warp_clip_v1(&source, &plan).unwrap();
        assert_eq!(candidate.tracks.len(), FOOT_CYCLE_CLIP_V1_MAX_TRACKS);
        assert!(candidate.tracks.iter().all(|track| {
            let value_capacity = match &track.values {
                TrackValues::Vec3s(values) => values.capacity(),
                TrackValues::Quats(values) => values.capacity(),
            };
            track.times.len() == 1
                && track.times.capacity() == 1
                && track.values.len() == 1
                && value_capacity == 1
        }));
    }

    #[test]
    fn nonunit_duration_scales_step_and_linear_inserted_knots() {
        let duration = 1.1_f64;
        let narrowed_duration = duration as f32;
        let source = clip(
            duration,
            vec![
                vec_track(
                    Interpolation::Step,
                    vec![0.0, narrowed_duration * 0.5, narrowed_duration],
                    vec![Vec3::ZERO, Vec3::ONE, Vec3::splat(2.0)],
                ),
                Track {
                    bone: 8,
                    property: Property::Scale,
                    interpolation: Interpolation::Linear,
                    times: vec![0.0, narrowed_duration * 0.5, narrowed_duration],
                    values: TrackValues::Vec3s(vec![
                        Vec3::ZERO,
                        Vec3::splat(10.0),
                        Vec3::splat(20.0),
                    ]),
                },
            ],
        );
        let candidate = time_warp_clip_v1(&source, &non_identity_plan(duration)).unwrap();
        let mapped_middle = 2.0 * narrowed_duration / 3.0;

        assert_eq!(candidate.duration_s.to_bits(), duration.to_bits());
        assert_eq!(candidate.tracks[0].times.len(), 3);
        assert_eq!(candidate.tracks[1].times.len(), 4);
        assert_eq!(candidate.tracks[0].times[0], 0.0);
        assert_approx(candidate.tracks[0].times[1], mapped_middle);
        assert_eq!(candidate.tracks[0].times[2], narrowed_duration);
        assert_eq!(candidate.tracks[1].times[0], 0.0);
        assert_approx(candidate.tracks[1].times[1], narrowed_duration * 0.5);
        assert_approx(candidate.tracks[1].times[2], mapped_middle);
        assert_eq!(candidate.tracks[1].times[3], narrowed_duration);
        assert_eq!(
            vec_values(&candidate.tracks[1]),
            &[
                Vec3::ZERO,
                Vec3::splat(5.0),
                Vec3::splat(10.0),
                Vec3::splat(20.0),
            ]
        );
    }

    #[test]
    fn non_affine_linear_map_maps_authored_keys_and_samples_interior_knots() {
        let source = clip(
            1.0,
            vec![vec_track(
                Interpolation::Linear,
                vec![0.0, 0.5, 1.0],
                vec![Vec3::ZERO, Vec3::splat(10.0), Vec3::splat(20.0)],
            )],
        );

        let candidate = time_warp_clip_v1(&source, &non_identity_plan(1.0)).unwrap();

        assert_eq!(candidate.name, "walk_forward");
        assert_eq!(candidate.duration_s, 1.0);
        assert_eq!(candidate.tracks[0].bone, 7);
        assert_eq!(candidate.tracks[0].property, Property::Translation);
        assert_eq!(candidate.tracks[0].interpolation, Interpolation::Linear);
        assert_eq!(candidate.tracks[0].times.len(), 4);
        assert_eq!(candidate.tracks[0].times[0], 0.0);
        assert_eq!(candidate.tracks[0].times[1], 0.5);
        assert_approx(candidate.tracks[0].times[2], 2.0 / 3.0);
        assert_eq!(candidate.tracks[0].times[3], 1.0);
        assert_eq!(
            vec_values(&candidate.tracks[0]),
            &[
                Vec3::ZERO,
                Vec3::splat(5.0),
                Vec3::splat(10.0),
                Vec3::splat(20.0)
            ]
        );
        assert_eq!(
            sample_track(&candidate.tracks[0], 0.5),
            sample_track(&source.tracks[0], 0.25)
        );
    }

    #[test]
    fn nonidentity_multi_track_candidate_preserves_order_and_metadata() {
        let zero_quat = Quat::from_xyzw(0.0, 0.0, 0.0, 0.0);
        let source = clip(
            1.0,
            vec![
                vec_track(
                    Interpolation::Linear,
                    vec![0.0, 1.0],
                    vec![Vec3::ZERO, Vec3::ONE],
                ),
                Track {
                    bone: 8,
                    property: Property::Scale,
                    interpolation: Interpolation::Step,
                    times: vec![0.0, 1.0],
                    values: TrackValues::Vec3s(vec![Vec3::ONE, Vec3::splat(2.0)]),
                },
                quat_track(
                    Interpolation::CubicSpline,
                    vec![0.0, 1.0],
                    vec![
                        zero_quat,
                        Quat::IDENTITY,
                        zero_quat,
                        zero_quat,
                        Quat::IDENTITY,
                        zero_quat,
                    ],
                ),
            ],
        );

        let candidate = time_warp_clip_v1(&source, &non_identity_plan(1.0)).unwrap();

        assert_eq!(
            candidate
                .tracks
                .iter()
                .map(|track| (track.bone, track.property, track.interpolation))
                .collect::<Vec<_>>(),
            vec![
                (7, Property::Translation, Interpolation::Linear),
                (8, Property::Scale, Interpolation::Step),
                (9, Property::Rotation, Interpolation::CubicSpline),
            ]
        );
    }

    #[test]
    fn linear_knot_coincident_with_authored_key_is_deduplicated_deterministically() {
        let source = clip(
            1.0,
            vec![vec_track(
                Interpolation::Linear,
                vec![0.0, 0.25, 1.0],
                vec![Vec3::ZERO, Vec3::ONE, Vec3::splat(2.0)],
            )],
        );
        let plan = non_identity_plan(1.0);

        let preflight = preflight_time_warp_clip_v1(&source, &plan).unwrap();

        let first = time_warp_clip_v1(&source, &plan).unwrap();
        let second = time_warp_clip_v1(&source, &plan).unwrap();

        assert_eq!(first.tracks[0].times, vec![0.0, 0.5, 1.0]);
        assert_eq!(preflight.candidate_keys(), first.tracks[0].times.len());
        assert_eq!(preflight.candidate_values(), first.tracks[0].values.len());
        assert_eq!(first.tracks[0].times, second.tracks[0].times);
        assert_eq!(vec_values(&first.tracks[0]), vec_values(&second.tracks[0]));
    }

    #[test]
    fn linear_knot_that_narrows_to_authored_key_is_deduplicated() {
        let duration = f64::from(17.0_f32 / 30.0);
        let authored_time = 0.1_f32;
        let control_phase = 3.0 / 17.0;
        let reconstructed_time = control_phase * duration;
        assert_ne!(reconstructed_time, f64::from(authored_time));
        assert_eq!(reconstructed_time as f32, authored_time);

        let source = clip(
            duration,
            vec![vec_track(
                Interpolation::Linear,
                vec![0.0, authored_time, duration as f32],
                vec![Vec3::ZERO, Vec3::ONE, Vec3::splat(2.0)],
            )],
        );
        let plan = plan(
            duration,
            &[
                (0.0, 0.0),
                (control_phase, control_phase),
                (0.5, 0.4),
                (1.0, 1.0),
            ],
        );

        let preflight = preflight_time_warp_clip_v1(&source, &plan).unwrap();
        let candidate = time_warp_clip_v1(&source, &plan).unwrap();

        assert_eq!(preflight.candidate_keys(), 4);
        assert_eq!(candidate.tracks[0].times.len(), preflight.candidate_keys());
        assert_eq!(
            candidate.tracks[0].values.len(),
            preflight.candidate_values()
        );
        assert_eq!(candidate.tracks[0].times[1], authored_time);
        assert_eq!(vec_values(&candidate.tracks[0])[1], Vec3::ONE);
        assert_eq!(
            candidate.tracks[0]
                .times
                .iter()
                .filter(|&&time| time == authored_time)
                .count(),
            1
        );
    }

    /// A control point that is the same instant as a source endpoint neither
    /// adds a key nor re-times the key there.
    ///
    /// The map's exact `(0,0)` and `(1,1)` rows define the output at `0` and
    /// at the duration. Letting an interior point win there would silently end
    /// the candidate track early, or start it late, without a refusal.
    #[test]
    fn interior_control_point_beside_a_source_endpoint_contributes_nothing() {
        let above_start = f64::from(f32::from_bits(1));
        let below_end = f64::from(f32::from_bits(1.0_f32.to_bits() - 1));
        let source = clip(
            1.0,
            vec![vec_track(
                Interpolation::Linear,
                vec![0.0, 0.5, 1.0],
                vec![Vec3::ZERO, Vec3::ONE, Vec3::splat(2.0)],
            )],
        );
        let plan = plan(
            1.0,
            &[(0.0, 0.0), (above_start, 0.2), (below_end, 0.8), (1.0, 1.0)],
        );

        let preflight = preflight_time_warp_clip_v1(&source, &plan).unwrap();
        let candidate = time_warp_clip_v1(&source, &plan).unwrap();

        assert_eq!(preflight.candidate_keys(), 3);
        assert_eq!(candidate.tracks[0].times.len(), 3);
        assert_eq!(
            candidate.tracks[0].times[0], 0.0,
            "the candidate still starts where the source does"
        );
        assert_eq!(
            candidate.tracks[0].times[2], 1.0,
            "the candidate still ends where the source does"
        );
        assert_eq!(
            vec_values(&candidate.tracks[0]),
            &[Vec3::ZERO, Vec3::ONE, Vec3::splat(2.0)]
        );
    }

    /// The public row sequence, exercised directly: it is what an embedder
    /// proving a candidate independently is told to consume.
    ///
    /// Rows arrive in source order and classify every emitted key; a knot that
    /// narrows onto `0` or the duration is dropped; and a track that emits no
    /// knots at all yields one `Authored` row per authored key.
    #[test]
    fn public_row_sequence_classifies_and_orders_the_emitted_keys() {
        let duration = 2.0_f32;
        let track = vec_track(
            Interpolation::Linear,
            vec![0.0, 0.5, 1.0, 1.5, 2.0],
            vec![Vec3::ZERO; 5],
        );
        let operation = ContactTransformOperationV1::time_warp(
            f64::from(duration),
            [
                (0.0, 0.0),
                // Narrows to 0.0, below the smallest binary32 subnormal.
                (1e-46, 0.05),
                // Narrows onto the authored key at 0.5.
                (0.25, 0.2),
                // Narrows between two authored keys.
                (0.375, 0.5),
                // Narrows to the duration: binary32 has nothing between.
                (1.0 - 1e-16, 0.95),
                (1.0, 1.0),
            ]
            .into_iter()
            .map(|(input, output)| point(input, output))
            .collect(),
        );
        let points = operation.control_points().unwrap();
        assert_eq!((1e-46 * f64::from(duration)) as f32, 0.0);
        assert_eq!(((1.0 - 1e-16) * f64::from(duration)) as f32, duration);

        let rows = time_warp_rows_v1(points, duration, &track).collect::<Vec<_>>();

        let classified = rows
            .iter()
            .map(|row| match row {
                FootCycleClipWarpRowV1::Authored(index) => ("authored", *index, f32::NAN),
                FootCycleClipWarpRowV1::Knot(knot) => ("knot", usize::MAX, knot.source_time()),
                FootCycleClipWarpRowV1::Coalesced(index, knot) => {
                    ("coalesced", *index, knot.source_time())
                }
            })
            .collect::<Vec<_>>();
        assert_eq!(
            classified
                .iter()
                .map(|(kind, index, _)| (*kind, *index))
                .collect::<Vec<_>>(),
            vec![
                ("authored", 0),
                ("coalesced", 1),
                ("knot", usize::MAX),
                ("authored", 2),
                ("authored", 3),
                ("authored", 4),
            ],
            "the two knots at the source endpoints are dropped"
        );
        assert_eq!(
            classified[1].2, 0.5,
            "the coalesced knot is the authored key"
        );
        assert_eq!(classified[2].2, 0.75);
        let coalesced_knot = match rows[1] {
            FootCycleClipWarpRowV1::Coalesced(_, knot) => knot,
            _ => panic!("row 1 is coalesced"),
        };
        assert_eq!(coalesced_knot.control_point_index(), 2);
        assert_eq!(coalesced_knot.output_time(), 0.4);

        for interpolation in [Interpolation::Step, Interpolation::CubicSpline] {
            let mut other = track.clone();
            other.interpolation = interpolation;
            assert_eq!(
                time_warp_rows_v1(points, duration, &other).collect::<Vec<_>>(),
                (0..other.times.len())
                    .map(FootCycleClipWarpRowV1::Authored)
                    .collect::<Vec<_>>(),
                "{interpolation:?} tracks emit only authored rows"
            );
        }
    }

    /// Two knots that name one instant through the same authored key refuse.
    ///
    /// Each is the same instant as the key, so only one can coalesce into it
    /// and the other would publish as its own key one place away — the plan
    /// naming one breakpoint twice, which is the collision the emitted domain
    /// cannot represent.
    #[test]
    fn two_knots_straddling_one_authored_key_refuse() {
        let interior = vec![0.0, 0.4, 0.6, 1.0];
        let span_end = vec![0.25, 0.5, 0.75];
        for (times, straddled, offsets) in [
            // One place on each side: each knot is the key, only one can be.
            (&interior, 0.4_f32, (-1_i32, 1_i32)),
            (&span_end, 0.75, (-1, 1)),
            // Both above the key, adjacent to each other: the first coalesces
            // into the key and answers to a time one place from its own, so
            // comparing the second only with that time would let it through.
            (&interior, 0.4, (1, 2)),
            // The mirror, both below.
            (&interior, 0.4, (-2, -1)),
            (&span_end, 0.75, (-2, -1)),
        ] {
            let shifted = |offset: i32| {
                f64::from(f32::from_bits(
                    straddled.to_bits().wrapping_add_signed(offset),
                ))
            };
            let source = clip(
                1.0,
                vec![vec_track(
                    Interpolation::Linear,
                    times.clone(),
                    vec![Vec3::ZERO; times.len()],
                )],
            );
            let plan = plan(
                1.0,
                &[
                    (0.0, 0.0),
                    (shifted(offsets.0), 0.2),
                    (shifted(offsets.1), 0.5),
                    (1.0, 1.0),
                ],
            );
            let points = plan.operation().control_points().unwrap();
            let knots = time_warp_rows_v1(points, 1.0, &source.tracks[0])
                .filter(|row| !matches!(row, FootCycleClipWarpRowV1::Authored(_)))
                .count();
            assert_eq!(
                knots, 2,
                "{straddled} {offsets:?}: both knots must reach the sequence"
            );

            assert_preflight_and_candidate_error(
                &source,
                &plan,
                FootCycleClipWarpError::SourceTimeCollision { track_index: 0 },
            );
        }
    }

    /// Two control points the emitted domain cannot tell apart refuse whether
    /// or not an authored key sits beside them.
    ///
    /// The pair is one instant there, so the plan names a breakpoint twice.
    /// Deciding that between the knots rather than between the emitted keys is
    /// what keeps the refusal independent of the source's own key times: with
    /// a key beside the pair the first knot would otherwise coalesce into it
    /// and the second would become an ordinary key one place away.
    #[test]
    fn two_control_points_that_are_one_instant_refuse_beside_a_key_or_not() {
        let shared = 0.5_f32;
        let values = vec![Vec3::ZERO, Vec3::ONE, Vec3::splat(2.0)];
        for beside_a_key in [true, false] {
            let middle = if beside_a_key { shared } else { 0.25 };
            for (label, second) in [
                ("the same instant", f64::from(shared)),
                ("adjacent instants", f64::from(shared.next_up())),
            ] {
                let source = clip(
                    1.0,
                    vec![vec_track(
                        Interpolation::Linear,
                        vec![0.0, middle, 1.0],
                        values.clone(),
                    )],
                );
                // A nudge too small to survive narrowing: both control points
                // resolve into the pair this test is about.
                let plan = plan(
                    1.0,
                    &[
                        (0.0, 0.0),
                        (f64::from(shared), 0.4),
                        (second + f64::EPSILON, 0.6),
                        (1.0, 1.0),
                    ],
                );
                let points = plan.operation().control_points().unwrap();
                let knots = time_warp_knots_v1(points, 1.0, &source.tracks[0]).collect::<Vec<_>>();
                assert_eq!(knots.len(), 2, "{label}, beside a key: {beside_a_key}");
                assert!(knots[1].coincides_with(knots[0].source_time()));

                assert_preflight_and_candidate_error(
                    &source,
                    &plan,
                    FootCycleClipWarpError::SourceTimeCollision { track_index: 0 },
                );
            }
        }
    }

    /// Two authored keys one binary32 place apart stay two instants, and a
    /// knot binds to the key it *is*, not to the key it is merely beside.
    ///
    /// Coalescing re-times an authored key; it never removes one. A rule that
    /// bound greedily to the earlier key would give the control point's output
    /// to the wrong one of the pair, and one that consumed every key it is
    /// beside would delete the neighbour.
    #[test]
    fn a_knot_binds_to_the_authored_key_it_equals_not_the_one_beside_it() {
        let first = 0.5_f32;
        let second = first.next_up();
        assert_ne!(first, second);

        for bound_index in [1_usize, 2] {
            let bound = if bound_index == 1 { first } else { second };
            let source = clip(
                1.0,
                vec![vec_track(
                    Interpolation::Linear,
                    vec![0.0, first, second, 1.0],
                    vec![Vec3::ZERO, Vec3::ONE, Vec3::splat(2.0), Vec3::splat(3.0)],
                )],
            );
            let plan = plan(1.0, &[(0.0, 0.0), (f64::from(bound), 0.25), (1.0, 1.0)]);
            let candidate = time_warp_clip_v1(&source, &plan).unwrap();
            let times = &candidate.tracks[0].times;

            assert_eq!(times.len(), 4, "both authored keys must be retained");
            assert_eq!(
                times[bound_index], 0.25,
                "the key the knot equals takes the control point's output"
            );
            let other = if bound_index == 1 { 2 } else { 1 };
            assert_ne!(
                times[other], 0.25,
                "the key beside it keeps its own mapped output"
            );
            assert_eq!(
                vec_values(&candidate.tracks[0]),
                &[Vec3::ZERO, Vec3::ONE, Vec3::splat(2.0), Vec3::splat(3.0)]
            );
        }
    }

    /// A knot beside a track's own span end coalesces from either side.
    ///
    /// The span test is inclusive, so all six placements around the first and
    /// last authored key — one place below, exactly on it, one place above —
    /// give that key its control point's output. Which side of a rounding step
    /// the reconstructed instant lands on is exactly what must not decide the
    /// outcome.
    #[test]
    fn a_knot_beside_a_track_span_end_coalesces_from_either_side() {
        for (key, emitted_index, before, after, outputs) in [
            (
                0.25_f32,
                0_usize,
                0.249_999_8,
                0.250_000_2,
                [0.07, 0.1, 0.13],
            ),
            (0.75, 3, 0.749_999_8, 0.750_000_2, [0.87, 0.9, 0.93]),
        ] {
            for offset in [-1_i32, 0, 1] {
                let control_time = f32::from_bits(key.to_bits().wrapping_add_signed(offset));
                let source = clip(
                    1.0,
                    vec![vec_track(
                        Interpolation::Linear,
                        vec![0.25, 0.5, 0.75],
                        vec![Vec3::ZERO, Vec3::ONE, Vec3::splat(2.0)],
                    )],
                );
                let plan = plan(
                    1.0,
                    &[
                        (0.0, 0.0),
                        (before, outputs[0]),
                        (f64::from(control_time), outputs[1]),
                        (after, outputs[2]),
                        (1.0, 1.0),
                    ],
                );
                let points = plan.operation().control_points().unwrap();
                if offset != 0 {
                    assert_ne!(
                        map_time(key, 1.0, points),
                        outputs[1] as f32,
                        "key {key} offset {offset}: the mapped output must differ"
                    );
                }

                let candidate = time_warp_clip_v1(&source, &plan).unwrap();
                let times = &candidate.tracks[0].times;

                assert_eq!(times.len(), 4, "key {key} offset {offset}");
                assert_eq!(
                    times[emitted_index], outputs[1] as f32,
                    "key {key} offset {offset}: the span end takes its control point's output"
                );
                assert!(times.windows(2).all(|pair| pair[0] < pair[1]));
            }
        }
    }

    /// The predicate's far edge: two places apart are two instants.
    ///
    /// A control point two binary32 places from an authored key keeps its own
    /// key, so the candidate carries both. Widening the predicate by one more
    /// place would coalesce them and lose a key.
    #[test]
    fn control_point_two_places_from_an_authored_key_keeps_its_own_key() {
        let authored_time = 0.5_f32;
        let two_below = f32::from_bits(authored_time.to_bits() - 2);
        assert_ne!(f32::from_bits(authored_time.to_bits() - 1), two_below);

        let source = clip(
            1.0,
            vec![vec_track(
                Interpolation::Linear,
                vec![0.0, authored_time, 1.0],
                vec![Vec3::ZERO, Vec3::ONE, Vec3::splat(2.0)],
            )],
        );
        let plan = plan(1.0, &[(0.0, 0.0), (f64::from(two_below), 0.25), (1.0, 1.0)]);
        let points = plan.operation().control_points().unwrap();
        let knot = time_warp_knots_v1(points, 1.0, &source.tracks[0])
            .next()
            .expect("one interior knot");
        assert_eq!(knot.source_time(), two_below);
        assert!(!knot.coincides_with(authored_time));

        let preflight = preflight_time_warp_clip_v1(&source, &plan).unwrap();
        let candidate = time_warp_clip_v1(&source, &plan).unwrap();
        let times = &candidate.tracks[0].times;

        assert_eq!(preflight.candidate_keys(), 4);
        assert_eq!(times.len(), 4);
        assert_eq!(times[1], 0.25, "the knot keeps its own key");
        assert!(
            times[2] > 0.25,
            "the authored key keeps its own mapped output"
        );
        assert_eq!(vec_values(&candidate.tracks[0])[2], Vec3::ONE);
    }

    /// A control point one binary32 place from an authored key is that key.
    ///
    /// Nothing is representable between the two, so a candidate that stored a
    /// key at each would carry two spellings of one instant. The pair is one
    /// emitted key: the authored time, the authored value, and the control
    /// point's own output.
    /// Ordinary locomotion sets coalesce every stance-boundary control point,
    /// so the candidate keeps exactly the authored key count.
    ///
    /// Stance boundaries are authored frame indices, so the plan's normalized
    /// input is `frame / (frames - 1)`. Reconstructing that instant as
    /// `input * duration` reproduces the authored binary32 time for most key
    /// counts and rates in this range and lands one place beside it — above it
    /// as well as below — for the rest; all of those are the same instant, so
    /// none of them adds a key.
    #[test]
    fn stance_boundaries_coalesce_for_every_key_count_and_rate() {
        for (rate, frames) in [24.0_f32, 30.0, 60.0]
            .into_iter()
            .flat_map(|rate| (18..=61_usize).map(move |frames| (rate, frames)))
        {
            let last = frames - 1;
            let duration = f64::from(last as f32 / rate);
            let source = clip(
                duration,
                vec![vec_track(
                    Interpolation::Linear,
                    (0..frames).map(|key| key as f32 / rate).collect(),
                    (0..frames).map(|key| Vec3::splat(key as f32)).collect(),
                )],
            );
            let phase = |frame: usize| frame as f64 / last as f64;
            let boundaries = [(4, 2), (6, 4), (last - 4, last - 6), (last - 2, last - 4)];
            let mut points = vec![(0.0, 0.0)];
            points.extend(
                boundaries
                    .iter()
                    .map(|&(input, output)| (phase(input), phase(output))),
            );
            points.push((1.0, 1.0));
            let plan = plan(duration, &points);

            let preflight = preflight_time_warp_clip_v1(&source, &plan).unwrap();
            let candidate = time_warp_clip_v1(&source, &plan).unwrap();
            let times = &candidate.tracks[0].times;

            assert_eq!(
                times.len(),
                frames,
                "{frames} keys at {rate} fps: coalescing must not add a key"
            );
            assert_eq!(preflight.candidate_keys(), frames);
            assert!(times.windows(2).all(|pair| pair[0] < pair[1]));
            for (input, output) in boundaries {
                assert_eq!(
                    times[input],
                    (phase(output) * duration) as f32,
                    "{frames} keys at {rate} fps: key {input} must carry its control point output"
                );
            }
            assert_eq!(
                vec_values(&candidate.tracks[0]),
                &(0..frames)
                    .map(|key| Vec3::splat(key as f32))
                    .collect::<Vec<_>>(),
                "{frames} keys at {rate} fps: every emitted key keeps its authored value"
            );
        }
    }

    #[test]
    fn adjacent_binary32_time_is_one_instant() {
        let authored_time = 0.5_f32;
        let adjacent_time = f32::from_bits(authored_time.to_bits() + 1);
        assert_ne!(authored_time, adjacent_time);

        let source = clip(
            1.0,
            vec![vec_track(
                Interpolation::Linear,
                vec![0.0, authored_time, 1.0],
                vec![Vec3::ZERO, Vec3::ONE, Vec3::splat(2.0)],
            )],
        );
        let plan = plan(
            1.0,
            &[(0.0, 0.0), (f64::from(adjacent_time), 0.75), (1.0, 1.0)],
        );

        let preflight = preflight_time_warp_clip_v1(&source, &plan).unwrap();
        let candidate = time_warp_clip_v1(&source, &plan).unwrap();

        assert_eq!(preflight.candidate_keys(), 3);
        assert_eq!(candidate.tracks[0].times.len(), preflight.candidate_keys());
        assert_eq!(
            candidate.tracks[0].values.len(),
            preflight.candidate_values()
        );
        assert_eq!(candidate.tracks[0].times, vec![0.0, 0.75, 1.0]);
        assert_eq!(vec_values(&candidate.tracks[0])[1], Vec3::ONE);
    }

    #[test]
    fn linear_knot_that_rounds_down_to_authored_key_is_deduplicated() {
        let authored_time = 0.5_f32;
        let next_time = f32::from_bits(authored_time.to_bits() + 1);
        let control_time =
            f64::from(authored_time) + (f64::from(next_time) - f64::from(authored_time)) * 0.25;
        assert!(control_time > f64::from(authored_time));
        assert_eq!(control_time as f32, authored_time);

        let source = clip(
            1.0,
            vec![vec_track(
                Interpolation::Linear,
                vec![0.0, authored_time, 1.0],
                vec![Vec3::ZERO, Vec3::ONE, Vec3::splat(2.0)],
            )],
        );
        let plan = plan(
            1.0,
            &[
                (0.0, 0.0),
                (control_time, control_time),
                (0.75, 0.7),
                (1.0, 1.0),
            ],
        );

        let preflight = preflight_time_warp_clip_v1(&source, &plan).unwrap();
        let candidate = time_warp_clip_v1(&source, &plan).unwrap();

        assert_eq!(candidate.tracks[0].times.len(), preflight.candidate_keys());
        assert_eq!(
            candidate.tracks[0].values.len(),
            preflight.candidate_values()
        );
        assert_eq!(candidate.tracks[0].times[1], authored_time);
        assert_eq!(vec_values(&candidate.tracks[0])[1], Vec3::ONE);
    }

    /// A control point that narrows onto an authored key is that key, and the
    /// emitted key carries the control point's own output.
    ///
    /// Before this contract the builder recomputed the map through the
    /// authored time and refused the ordinary one-place difference with
    /// `TimeCollision`, which is what stopped every 30 fps locomotion set from
    /// publishing.
    #[test]
    fn narrowed_authored_key_takes_the_control_point_output() {
        let duration = f64::from(17.0_f32 / 30.0);
        let authored_time = 0.1_f32;
        let control_phase = 3.0 / 17.0;
        assert_ne!(control_phase * duration, f64::from(authored_time));
        assert_eq!((control_phase * duration) as f32, authored_time);
        let source = clip(
            duration,
            vec![vec_track(
                Interpolation::Linear,
                vec![0.0, authored_time, duration as f32],
                vec![Vec3::ZERO, Vec3::ONE, Vec3::splat(2.0)],
            )],
        );
        let plan = plan(
            duration,
            &[(0.0, 0.0), (control_phase, 4.0 / 17.0), (1.0, 1.0)],
        );
        let points = plan.operation().control_points().unwrap();
        let output_time = (4.0 / 17.0 * duration) as f32;
        assert_ne!(
            map_time(authored_time, duration as f32, points),
            output_time
        );

        let preflight = preflight_time_warp_clip_v1(&source, &plan).unwrap();
        let candidate = time_warp_clip_v1(&source, &plan).unwrap();

        assert_eq!(preflight.candidate_keys(), 3);
        assert_eq!(candidate.tracks[0].times.len(), 3);
        assert_eq!(candidate.tracks[0].times[1], output_time);
        assert_eq!(vec_values(&candidate.tracks[0])[1], Vec3::ONE);
    }

    /// The coalesced key carries the control point's output, not the map
    /// recomputed through the authored time.
    ///
    /// The two differ here by much more than a rounding step, so a builder
    /// that kept the mapped authored output would fail this test.
    #[test]
    fn coalesced_key_takes_the_control_point_output_not_the_mapped_one() {
        let authored_time = 0.5_f32;
        let next_time = f32::from_bits(authored_time.to_bits() + 1);
        let control_time =
            f64::from(authored_time) + (f64::from(next_time) - f64::from(authored_time)) * 0.49;
        assert!(control_time > f64::from(authored_time));
        assert_eq!(control_time as f32, authored_time);

        let source = clip(
            1.0,
            vec![vec_track(
                Interpolation::Linear,
                vec![0.0, authored_time, 1.0],
                vec![Vec3::ZERO, Vec3::ONE, Vec3::splat(2.0)],
            )],
        );
        let plan = plan(1.0, &[(0.0, 0.0), (control_time, 0.75), (1.0, 1.0)]);
        let points = plan.operation().control_points().unwrap();
        assert!(map_time(authored_time, 1.0, points) < 0.75);

        let preflight = preflight_time_warp_clip_v1(&source, &plan).unwrap();
        let candidate = time_warp_clip_v1(&source, &plan).unwrap();

        assert_eq!(preflight.candidate_keys(), 3);
        assert_eq!(candidate.tracks[0].times, vec![0.0, 0.75, 1.0]);
        assert_eq!(vec_values(&candidate.tracks[0])[1], Vec3::ONE);
    }

    /// Coalescing holds at the extremes of the binary32 time domain: a
    /// multi-megasecond duration and a subnormal-scale output still emit one
    /// strictly increasing key per instant.
    #[test]
    fn coincident_source_knot_coalesces_at_extreme_magnitudes() {
        let duration = 12_794_115.0;
        let source = clip(
            duration,
            vec![vec_track(
                Interpolation::Linear,
                vec![0.0, 5_496_923.0, 12_794_115.0],
                vec![Vec3::ZERO, Vec3::ONE, Vec3::splat(2.0)],
            )],
        );
        let plan = plan(
            duration,
            &[
                (0.0, 0.0),
                (0.429_644_645_213_834_6, 7.523_163_845_262_64e-37),
                (1.0, 1.0),
            ],
        );
        let output_time = (7.523_163_845_262_64e-37 * duration) as f32;

        let preflight = preflight_time_warp_clip_v1(&source, &plan).unwrap();
        let candidate = time_warp_clip_v1(&source, &plan).unwrap();

        assert_eq!(preflight.candidate_keys(), 3);
        assert_eq!(
            candidate.tracks[0].times,
            vec![0.0, output_time, duration as f32]
        );
        assert_eq!(vec_values(&candidate.tracks[0])[1], Vec3::ONE);
    }

    #[test]
    fn step_maps_only_authored_breakpoints_and_preserves_hold() {
        let source = clip(
            1.0,
            vec![vec_track(
                Interpolation::Step,
                vec![0.0, 0.5, 1.0],
                vec![Vec3::ZERO, Vec3::ONE, Vec3::splat(2.0)],
            )],
        );

        let candidate = time_warp_clip_v1(&source, &non_identity_plan(1.0)).unwrap();

        assert_eq!(candidate.tracks[0].times.len(), 3);
        assert_eq!(candidate.tracks[0].times[0], 0.0);
        assert_approx(candidate.tracks[0].times[1], 2.0 / 3.0);
        assert_eq!(candidate.tracks[0].times[2], 1.0);
        assert_eq!(
            sample_track(&candidate.tracks[0], 0.6),
            TrackSample::Vec3(Vec3::ZERO)
        );
    }

    #[test]
    fn identity_map_is_structurally_identical_for_admissible_tracks() {
        let mut cubic = vec_track(
            Interpolation::CubicSpline,
            vec![0.0, 1.0],
            vec![
                Vec3::ZERO,
                Vec3::ONE,
                Vec3::ZERO,
                Vec3::ZERO,
                Vec3::ONE,
                Vec3::ZERO,
            ],
        );
        cubic.bone = 10;
        let source = clip(
            1.0,
            vec![
                vec_track(
                    Interpolation::Linear,
                    vec![0.0, 1.0],
                    vec![Vec3::ZERO, Vec3::ONE],
                ),
                Track {
                    bone: 8,
                    property: Property::Scale,
                    interpolation: Interpolation::Step,
                    times: vec![0.2, 0.8],
                    values: TrackValues::Vec3s(vec![Vec3::ONE, Vec3::splat(2.0)]),
                },
                cubic,
            ],
        );
        let before = source.clone();

        let candidate =
            time_warp_clip_v1(&source, &plan(1.0, &[(0.0, 0.0), (0.4, 0.4), (1.0, 1.0)])).unwrap();

        assert_clip_bits_equal(&candidate, &before);
        assert_clip_bits_equal(&source, &before);
    }

    #[test]
    fn non_exact_binary32_duration_is_permitted_and_preserved_as_binary64() {
        let source = clip(
            0.1,
            vec![vec_track(
                Interpolation::Step,
                vec![0.0, 0.1_f32],
                vec![Vec3::ZERO, Vec3::ONE],
            )],
        );
        let candidate =
            time_warp_clip_v1(&source, &plan(0.1, &[(0.0, 0.0), (0.5, 0.4), (1.0, 1.0)])).unwrap();

        assert_eq!(candidate.duration_s.to_bits(), 0.1_f64.to_bits());
        assert_eq!(candidate.tracks[0].times.last(), Some(&0.1_f32));
    }

    #[test]
    fn constant_vec3_and_quaternion_cubic_tracks_are_retained_exactly() {
        let zero_quat = Quat::from_xyzw(0.0, 0.0, 0.0, 0.0);
        let vec_value = Vec3::new(1.0, 2.0, 3.0);
        let quat_value = Quat::from_xyzw(0.0, 0.0, 0.5, 0.5);
        let source = clip(
            1.0,
            vec![
                vec_track(
                    Interpolation::CubicSpline,
                    vec![0.0, 1.0],
                    vec![
                        Vec3::ZERO,
                        vec_value,
                        Vec3::ZERO,
                        Vec3::ZERO,
                        vec_value,
                        Vec3::ZERO,
                    ],
                ),
                quat_track(
                    Interpolation::CubicSpline,
                    vec![0.0, 1.0],
                    vec![
                        zero_quat, quat_value, zero_quat, zero_quat, quat_value, zero_quat,
                    ],
                ),
            ],
        );
        let before = format!("{:?}", source.tracks);

        let candidate = time_warp_clip_v1(&source, &non_identity_plan(1.0)).unwrap();

        assert_eq!(format!("{:?}", candidate.tracks), before);
    }

    #[test]
    fn one_key_cubic_track_is_retained_without_tangent_restrictions() {
        let track = vec_track(
            Interpolation::CubicSpline,
            vec![0.5],
            vec![Vec3::splat(2.0), Vec3::ONE, Vec3::splat(3.0)],
        );
        let source = clip(1.0, vec![track]);
        let before = format!("{:?}", source.tracks[0]);

        let candidate = time_warp_clip_v1(&source, &non_identity_plan(1.0)).unwrap();

        assert_eq!(format!("{:?}", candidate.tracks[0]), before);
    }

    #[test]
    fn varying_or_nonzero_tangent_cubic_tracks_refuse_atomically() {
        let cases = [
            (
                vec![
                    Vec3::ZERO,
                    Vec3::ONE,
                    Vec3::ZERO,
                    Vec3::ZERO,
                    Vec3::splat(2.0),
                    Vec3::ZERO,
                ],
                FootCycleCubicSplineRefusalV1::DifferingValues,
            ),
            (
                vec![
                    Vec3::ZERO,
                    Vec3::ONE,
                    Vec3::ONE,
                    Vec3::ZERO,
                    Vec3::ONE,
                    Vec3::ZERO,
                ],
                FootCycleCubicSplineRefusalV1::NonZeroTangent,
            ),
        ];
        for (values, reason) in cases {
            let source = clip(
                1.0,
                vec![vec_track(
                    Interpolation::CubicSpline,
                    vec![0.0, 1.0],
                    values,
                )],
            );
            let before = format!("{source:?}");
            assert_error(
                time_warp_clip_v1(&source, &non_identity_plan(1.0)),
                FootCycleClipWarpError::UnsupportedCubicSpline {
                    track_index: 0,
                    reason,
                },
            );
            assert_eq!(format!("{source:?}"), before);
            let identity = plan(1.0, &[(0.0, 0.0), (1.0, 1.0)]);
            assert_error(
                time_warp_clip_v1(&source, &identity),
                FootCycleClipWarpError::UnsupportedCubicSpline {
                    track_index: 0,
                    reason,
                },
            );
        }
    }

    #[test]
    fn quaternion_cubic_value_and_both_tangent_directions_refuse_atomically() {
        let zero = Quat::from_xyzw(0.0, 0.0, 0.0, 0.0);
        let value = Quat::IDENTITY;
        let different = Quat::from_rotation_y(0.5);
        let cases = [
            (
                vec![zero, value, zero, zero, different, zero],
                FootCycleCubicSplineRefusalV1::DifferingValues,
            ),
            (
                vec![Quat::IDENTITY, value, zero, zero, value, zero],
                FootCycleCubicSplineRefusalV1::NonZeroTangent,
            ),
            (
                vec![zero, value, Quat::IDENTITY, zero, value, zero],
                FootCycleCubicSplineRefusalV1::NonZeroTangent,
            ),
        ];
        for (values, reason) in cases {
            let source = clip(
                1.0,
                vec![quat_track(
                    Interpolation::CubicSpline,
                    vec![0.0, 1.0],
                    values,
                )],
            );
            let before = format!("{source:?}");
            assert_error(
                time_warp_clip_v1(&source, &non_identity_plan(1.0)),
                FootCycleClipWarpError::UnsupportedCubicSpline {
                    track_index: 0,
                    reason,
                },
            );
            assert_eq!(format!("{source:?}"), before);
        }
    }

    #[test]
    fn source_and_output_binary32_collisions_refuse() {
        let linear = clip(
            1.0,
            vec![vec_track(
                Interpolation::Linear,
                vec![0.0, 1.0],
                vec![Vec3::ZERO, Vec3::ONE],
            )],
        );
        let source_collision = plan(
            1.0,
            &[(0.0, 0.0), (0.5, 0.4), (0.500_000_001, 0.6), (1.0, 1.0)],
        );
        assert_preflight_and_candidate_error(
            &linear,
            &source_collision,
            FootCycleClipWarpError::SourceTimeCollision { track_index: 0 },
        );

        // The refusal names the offending track, not the first one.
        let mut second = vec_track(
            Interpolation::Linear,
            vec![0.0, 1.0],
            vec![Vec3::ZERO, Vec3::ONE],
        );
        second.bone += 1;
        let after_a_step_track = clip(
            1.0,
            vec![
                vec_track(
                    Interpolation::Step,
                    vec![0.0, 1.0],
                    vec![Vec3::ZERO, Vec3::ONE],
                ),
                second,
            ],
        );
        assert_preflight_and_candidate_error(
            &after_a_step_track,
            &source_collision,
            FootCycleClipWarpError::SourceTimeCollision { track_index: 1 },
        );

        let step = clip(
            1.0,
            vec![vec_track(
                Interpolation::Step,
                vec![0.0, 0.25, 0.5, 1.0],
                vec![Vec3::ZERO, Vec3::ONE, Vec3::splat(2.0), Vec3::splat(3.0)],
            )],
        );
        let output_collision = plan(
            1.0,
            &[(0.0, 0.0), (0.25, 0.5), (0.5, 0.500_000_001), (1.0, 1.0)],
        );
        assert_preflight_and_candidate_error(
            &step,
            &output_collision,
            FootCycleClipWarpError::TimeCollision { track_index: 0 },
        );
    }

    #[test]
    fn preflight_refuses_linear_mapped_key_collision() {
        let source = clip(
            1.0,
            vec![vec_track(
                Interpolation::Linear,
                vec![0.0, 0.25, 0.5, 1.0],
                vec![Vec3::ZERO, Vec3::ONE, Vec3::splat(2.0), Vec3::splat(3.0)],
            )],
        );
        assert_preflight_and_candidate_error(
            &source,
            &plan(
                1.0,
                &[(0.0, 0.0), (0.25, 0.5), (0.5, 0.500_000_001), (1.0, 1.0)],
            ),
            FootCycleClipWarpError::TimeCollision { track_index: 0 },
        );
    }

    #[test]
    fn malformed_duplicate_and_out_of_range_tracks_refuse_without_mutation() {
        let malformed = [
            vec_track(Interpolation::Linear, vec![], vec![]),
            vec_track(Interpolation::Linear, vec![f32::NAN], vec![Vec3::ZERO]),
            vec_track(
                Interpolation::Linear,
                vec![0.5, 0.5],
                vec![Vec3::ZERO, Vec3::ONE],
            ),
            vec_track(Interpolation::Linear, vec![0.0, 1.0], vec![Vec3::ZERO]),
            vec_track(
                Interpolation::Linear,
                vec![0.0],
                vec![Vec3::splat(f32::INFINITY)],
            ),
        ];
        let expected = [
            TrackShapeViolation::EmptyTimes,
            TrackShapeViolation::NonFiniteTime,
            TrackShapeViolation::TimesNotStrictlyIncreasing,
            TrackShapeViolation::ValueCountMismatch,
            TrackShapeViolation::NonFiniteValue,
        ];
        for (track, violation) in malformed.into_iter().zip(expected) {
            let source = clip(1.0, vec![track]);
            let before = format!("{source:?}");
            assert_error(
                time_warp_clip_v1(&source, &non_identity_plan(1.0)),
                FootCycleClipWarpError::InvalidTrack {
                    track_index: 0,
                    source: DocumentShapeError::TrackShape {
                        clip_index: 0,
                        node: 7,
                        violation,
                    },
                },
            );
            assert_eq!(format!("{source:?}"), before);
        }

        let track = vec_track(
            Interpolation::Step,
            vec![0.0, 1.1],
            vec![Vec3::ZERO, Vec3::ONE],
        );
        let source = clip(1.0, vec![track]);
        assert!(matches!(
            time_warp_clip_v1(&source, &non_identity_plan(1.0)),
            Err(FootCycleClipWarpError::TrackTimeOutOfRange { key_index: 1, .. })
        ));

        let negative = clip(
            1.0,
            vec![vec_track(
                Interpolation::Step,
                vec![-f32::MIN_POSITIVE, 0.5],
                vec![Vec3::ZERO, Vec3::ONE],
            )],
        );
        assert!(matches!(
            time_warp_clip_v1(&negative, &non_identity_plan(1.0)),
            Err(FootCycleClipWarpError::TrackTimeOutOfRange { key_index: 0, .. })
        ));

        let duplicate = vec_track(Interpolation::Step, vec![0.0], vec![Vec3::ZERO]);
        let source = clip(1.0, vec![duplicate.clone(), duplicate]);
        assert_error(
            time_warp_clip_v1(&source, &non_identity_plan(1.0)),
            FootCycleClipWarpError::DuplicateTrackTarget {
                track_index: 1,
                bone: 7,
                property: Property::Translation,
            },
        );
    }

    #[test]
    fn unsupported_operation_duration_and_map_structures_refuse() {
        let source = clip(
            1.0,
            vec![vec_track(Interpolation::Step, vec![0.0], vec![Vec3::ZERO])],
        );
        let unsupported = clip_test_member_plan(ContactTransformOperationV1::trim(
            ContactTransformIntervalV1::new(0.0, 1.0),
        ));
        assert_error(
            time_warp_clip_v1(&source, &unsupported),
            FootCycleClipWarpError::UnsupportedOperation,
        );
        let distinct_duration = f64::from_bits(1.0_f64.to_bits() + 1);
        assert_error(
            time_warp_clip_v1(&source, &plan(distinct_duration, &[(0.0, 0.0), (1.0, 1.0)])),
            FootCycleClipWarpError::DurationMismatch {
                clip_duration_s: 1.0,
                operation_duration_s: distinct_duration,
            },
        );
        let bad_version = clip_test_member_plan(ContactTransformOperationV1::TimeWarp {
            version: 2,
            output_duration_s: 1.0,
            control_points: vec![point(0.0, 0.0), point(1.0, 1.0)],
        });
        assert_error(
            time_warp_clip_v1(&source, &bad_version),
            FootCycleClipWarpError::UnsupportedVersion { version: 2 },
        );
        assert_error(
            time_warp_clip_v1(&source, &plan(1.0, &[(0.1, 0.0), (1.0, 1.0)])),
            FootCycleClipWarpError::InvalidMapEndpoints,
        );
        assert_error(
            time_warp_clip_v1(&source, &plan(1.0, &[(0.0, 0.0), (0.9, 1.0)])),
            FootCycleClipWarpError::InvalidMapEndpoints,
        );
        assert_error(
            time_warp_clip_v1(
                &source,
                &plan(1.0, &[(0.0, 0.0), (0.5, 0.6), (0.4, 0.7), (1.0, 1.0)]),
            ),
            FootCycleClipWarpError::NonMonotoneMap { index: 2 },
        );
        assert_error(
            time_warp_clip_v1(
                &source,
                &plan(1.0, &[(0.0, 0.0), (0.4, 0.7), (0.5, 0.6), (1.0, 1.0)]),
            ),
            FootCycleClipWarpError::NonMonotoneMap { index: 2 },
        );
        assert_error(
            time_warp_clip_v1(
                &source,
                &plan(1.0, &[(0.0, 0.0), (f64::NAN, 0.5), (1.0, 1.0)]),
            ),
            FootCycleClipWarpError::InvalidControlPoint { index: 1 },
        );
        assert_error(
            time_warp_clip_v1(
                &source,
                &plan(1.0, &[(0.0, 0.0), (0.5, 1.0 + f64::EPSILON), (1.0, 1.0)]),
            ),
            FootCycleClipWarpError::InvalidControlPoint { index: 1 },
        );
    }

    #[test]
    fn invalid_durations_control_point_counts_and_value_types_refuse() {
        let source = clip(
            1.0,
            vec![vec_track(Interpolation::Step, vec![0.0], vec![Vec3::ZERO])],
        );
        for duration_s in [-1.0, 0.0, f64::NAN, f64::MAX] {
            let invalid = clip(duration_s, source.tracks.clone());
            assert!(matches!(
                time_warp_clip_v1(&invalid, &plan(duration_s, &[(0.0, 0.0), (1.0, 1.0)])),
                Err(FootCycleClipWarpError::InvalidClipDuration { .. })
            ));
        }

        assert_error(
            time_warp_clip_v1(&source, &plan(1.0, &[(0.0, 0.0)])),
            FootCycleClipWarpError::InvalidControlPointCount {
                found: 1,
                maximum: crate::CONTACT_TRANSFORM_RESULT_V1_MAX_CONTROL_POINTS,
            },
        );
        let max_points = dense_points(crate::CONTACT_TRANSFORM_RESULT_V1_MAX_CONTROL_POINTS, true);
        assert!(time_warp_clip_v1(&source, &plan(1.0, &max_points)).is_ok());
        let too_many = dense_points(
            crate::CONTACT_TRANSFORM_RESULT_V1_MAX_CONTROL_POINTS + 1,
            true,
        );
        assert_error(
            time_warp_clip_v1(&source, &plan(1.0, &too_many)),
            FootCycleClipWarpError::InvalidControlPointCount {
                found: crate::CONTACT_TRANSFORM_RESULT_V1_MAX_CONTROL_POINTS + 1,
                maximum: crate::CONTACT_TRANSFORM_RESULT_V1_MAX_CONTROL_POINTS,
            },
        );

        let wrong_type = clip(
            1.0,
            vec![Track {
                bone: 0,
                property: Property::Rotation,
                interpolation: Interpolation::Linear,
                times: vec![0.0],
                values: TrackValues::Vec3s(vec![Vec3::ZERO]),
            }],
        );
        assert!(matches!(
            time_warp_clip_v1(&wrong_type, &non_identity_plan(1.0)),
            Err(FootCycleClipWarpError::InvalidTrack {
                source: DocumentShapeError::TrackShape {
                    violation: TrackShapeViolation::ValueTypeMismatchesProperty,
                    ..
                },
                ..
            })
        ));
    }

    #[test]
    fn finite_zero_quaternion_refuses_before_identity_or_inserted_knot_sampling() {
        let source = clip(
            1.0,
            vec![quat_track(
                Interpolation::Linear,
                vec![0.0, 1.0],
                vec![
                    Quat::from_xyzw(0.0, 0.0, 0.0, 0.0),
                    Quat::from_xyzw(0.0, 0.0, 0.0, 0.0),
                ],
            )],
        );
        let before = format!("{source:?}");

        for candidate_plan in [plan(1.0, &[(0.0, 0.0), (1.0, 1.0)]), non_identity_plan(1.0)] {
            assert_error(
                time_warp_clip_v1(&source, &candidate_plan),
                FootCycleClipWarpError::InvalidQuaternionKey {
                    track_index: 0,
                    key_index: 0,
                },
            );
        }
        assert_eq!(format!("{source:?}"), before);

        let zero = Quat::from_xyzw(0.0, 0.0, 0.0, 0.0);
        let cubic = clip(
            1.0,
            vec![quat_track(
                Interpolation::CubicSpline,
                vec![0.0, 1.0],
                vec![zero, zero, zero, zero, zero, zero],
            )],
        );
        assert_error(
            time_warp_clip_v1(&cubic, &plan(1.0, &[(0.0, 0.0), (1.0, 1.0)])),
            FootCycleClipWarpError::InvalidQuaternionKey {
                track_index: 0,
                key_index: 0,
            },
        );
    }

    #[test]
    fn extreme_finite_linear_vec3_and_quaternion_samples_remain_finite() {
        let magnitude = f32::MAX.sqrt() / 4.0;
        let source = clip(
            1.0,
            vec![
                vec_track(
                    Interpolation::Linear,
                    vec![0.0, 1.0],
                    vec![Vec3::splat(-f32::MAX), Vec3::splat(f32::MAX)],
                ),
                Track {
                    bone: 8,
                    property: Property::Rotation,
                    interpolation: Interpolation::Linear,
                    times: vec![0.0, 1.0],
                    values: TrackValues::Quats(vec![
                        Quat::from_xyzw(magnitude, 0.0, 0.0, 0.0),
                        Quat::from_xyzw(0.0, magnitude, 0.0, 0.0),
                    ]),
                },
            ],
        );

        let candidate = time_warp_clip_v1(&source, &non_identity_plan(1.0)).unwrap();

        assert!(
            vec_values(&candidate.tracks[0])
                .iter()
                .all(|value| value.is_finite())
        );
        assert!(
            quat_values(&candidate.tracks[1])
                .iter()
                .all(|value| value.is_finite())
        );
        assert!(
            quat_values(&candidate.tracks[1])[1]
                .length_squared()
                .is_finite()
        );
    }

    #[test]
    fn track_limit_is_enforced_at_exact_n_and_n_plus_one_through_public_api() {
        let make_clip = |count: usize| {
            clip(
                1.0,
                (0..count)
                    .map(|bone| Track {
                        bone,
                        property: Property::Translation,
                        interpolation: Interpolation::Step,
                        times: vec![0.0],
                        values: TrackValues::Vec3s(vec![Vec3::ZERO]),
                    })
                    .collect(),
            )
        };
        let identity = plan(1.0, &[(0.0, 0.0), (1.0, 1.0)]);
        assert_eq!(
            time_warp_clip_v1(&make_clip(FOOT_CYCLE_CLIP_V1_MAX_TRACKS), &identity)
                .unwrap()
                .tracks
                .len(),
            FOOT_CYCLE_CLIP_V1_MAX_TRACKS
        );
        assert_error(
            time_warp_clip_v1(&make_clip(FOOT_CYCLE_CLIP_V1_MAX_TRACKS + 1), &identity),
            FootCycleClipWarpError::LimitExceeded {
                resource: FootCycleClipResourceV1::Tracks,
                observed: FOOT_CYCLE_CLIP_V1_MAX_TRACKS + 1,
                maximum: FOOT_CYCLE_CLIP_V1_MAX_TRACKS,
            },
        );
    }

    #[test]
    fn every_aggregate_limit_has_exact_n_n_plus_one_and_overflow_coverage() {
        let limits = [
            (
                FootCycleClipResourceV1::Tracks,
                FOOT_CYCLE_CLIP_V1_MAX_TRACKS,
            ),
            (
                FootCycleClipResourceV1::InputKeys,
                FOOT_CYCLE_CLIP_V1_MAX_INPUT_KEYS,
            ),
            (
                FootCycleClipResourceV1::InputValues,
                FOOT_CYCLE_CLIP_V1_MAX_INPUT_VALUES,
            ),
            (
                FootCycleClipResourceV1::GeneratedKeys,
                FOOT_CYCLE_CLIP_V1_MAX_GENERATED_KEYS,
            ),
            (FootCycleClipResourceV1::Work, FOOT_CYCLE_CLIP_V1_MAX_WORK),
            (
                FootCycleClipResourceV1::NameBytes,
                FOOT_CYCLE_CLIP_V1_MAX_NAME_BYTES,
            ),
        ];
        for (resource, maximum) in limits {
            assert_eq!(check_limit(resource, maximum, maximum), Ok(()));
            assert_eq!(
                check_limit(resource, maximum + 1, maximum),
                Err(FootCycleClipWarpError::LimitExceeded {
                    resource,
                    observed: maximum + 1,
                    maximum,
                })
            );
            assert_eq!(
                checked_add(resource, usize::MAX, 1),
                Err(FootCycleClipWarpError::CountOverflow { resource })
            );
        }
    }

    #[test]
    fn input_value_limit_is_inclusive_through_public_api_before_shape_scan() {
        assert_eq!(
            FOOT_CYCLE_CLIP_V1_MAX_INPUT_VALUES,
            3 * FOOT_CYCLE_CLIP_V1_MAX_INPUT_KEYS
        );
        let exact = clip(
            1.0,
            vec![Track {
                bone: 0,
                property: Property::Translation,
                interpolation: Interpolation::Linear,
                times: vec![0.0],
                values: TrackValues::Vec3s(vec![Vec3::ZERO; FOOT_CYCLE_CLIP_V1_MAX_INPUT_VALUES]),
            }],
        );
        assert!(matches!(
            time_warp_clip_v1(&exact, &plan(1.0, &[(0.0, 0.0), (1.0, 1.0)])),
            Err(FootCycleClipWarpError::InvalidTrack { .. })
        ));

        let first_excess = clip(
            1.0,
            vec![Track {
                bone: 0,
                property: Property::Translation,
                interpolation: Interpolation::Linear,
                times: vec![0.0],
                values: TrackValues::Vec3s(vec![
                    Vec3::ZERO;
                    FOOT_CYCLE_CLIP_V1_MAX_INPUT_VALUES + 1
                ]),
            }],
        );

        assert_error(
            time_warp_clip_v1(&first_excess, &plan(1.0, &[(0.0, 0.0), (1.0, 1.0)])),
            FootCycleClipWarpError::LimitExceeded {
                resource: FootCycleClipResourceV1::InputValues,
                observed: FOOT_CYCLE_CLIP_V1_MAX_INPUT_VALUES + 1,
                maximum: FOOT_CYCLE_CLIP_V1_MAX_INPUT_VALUES,
            },
        );
    }

    #[test]
    fn input_and_generated_key_bounds_are_observable_at_n_and_n_plus_one() {
        let exact = clip(
            1.0,
            vec![dense_vec_track(
                Interpolation::Linear,
                FOOT_CYCLE_CLIP_V1_MAX_INPUT_KEYS,
            )],
        );
        let exact_candidate =
            time_warp_clip_v1(&exact, &plan(1.0, &[(0.0, 0.0), (1.0, 1.0)])).unwrap();
        assert_eq!(
            exact_candidate.tracks[0].times.len(),
            FOOT_CYCLE_CLIP_V1_MAX_INPUT_KEYS
        );
        assert_eq!(
            time_warp_clip_v1(&exact, &plan(1.0, &[(0.0, 0.0), (0.5, 0.5), (1.0, 1.0)]),)
                .unwrap()
                .tracks[0]
                .times
                .len(),
            FOOT_CYCLE_CLIP_V1_MAX_INPUT_KEYS
        );

        let generated_n_plus_one = plan(1.0, &[(0.0, 0.0), (0.5, 0.4), (1.0, 1.0)]);
        assert_error(
            time_warp_clip_v1(&exact, &generated_n_plus_one),
            FootCycleClipWarpError::LimitExceeded {
                resource: FootCycleClipResourceV1::GeneratedKeys,
                observed: FOOT_CYCLE_CLIP_V1_MAX_GENERATED_KEYS + 1,
                maximum: FOOT_CYCLE_CLIP_V1_MAX_GENERATED_KEYS,
            },
        );

        let input_n_plus_one = clip(
            1.0,
            vec![dense_vec_track(
                Interpolation::Step,
                FOOT_CYCLE_CLIP_V1_MAX_INPUT_KEYS + 1,
            )],
        );
        assert_error(
            time_warp_clip_v1(&input_n_plus_one, &plan(1.0, &[(0.0, 0.0), (1.0, 1.0)])),
            FootCycleClipWarpError::LimitExceeded {
                resource: FootCycleClipResourceV1::InputKeys,
                observed: FOOT_CYCLE_CLIP_V1_MAX_INPUT_KEYS + 1,
                maximum: FOOT_CYCLE_CLIP_V1_MAX_INPUT_KEYS,
            },
        );
    }

    #[test]
    fn work_bound_is_observable_at_exact_n_and_n_plus_one() {
        let work_clip = |linear_tracks: usize, step_keys: usize| {
            let mut tracks: Vec<Track> = (0..linear_tracks)
                .map(|bone| Track {
                    bone,
                    property: Property::Translation,
                    interpolation: Interpolation::Linear,
                    times: vec![0.5],
                    values: TrackValues::Vec3s(vec![Vec3::ZERO]),
                })
                .collect();
            tracks.push(Track {
                bone: linear_tracks,
                property: Property::Translation,
                interpolation: Interpolation::Step,
                times: (0..step_keys)
                    .map(|index| index as f32 / (step_keys - 1).max(1) as f32)
                    .collect(),
                values: TrackValues::Vec3s(vec![Vec3::ZERO; step_keys]),
            });
            clip(1.0, tracks)
        };

        let exact = work_clip(2_047, 1);
        let exact_points = dense_points(4_096, false);
        assert_eq!(
            time_warp_clip_v1(&exact, &plan(1.0, &exact_points))
                .unwrap()
                .tracks
                .len(),
            2_048
        );

        let n_plus_one = work_clip(2_047, 1_025);
        let n_plus_one_points = dense_points(4_095, false);
        assert_error(
            time_warp_clip_v1(&n_plus_one, &plan(1.0, &n_plus_one_points)),
            FootCycleClipWarpError::LimitExceeded {
                resource: FootCycleClipResourceV1::Work,
                observed: FOOT_CYCLE_CLIP_V1_MAX_WORK + 1,
                maximum: FOOT_CYCLE_CLIP_V1_MAX_WORK,
            },
        );
    }

    #[test]
    fn quaternion_linear_interior_knot_uses_existing_shortest_path_sampler() {
        let source = clip(
            1.0,
            vec![quat_track(
                Interpolation::Linear,
                vec![0.0, 1.0],
                vec![Quat::IDENTITY, -Quat::from_rotation_y(std::f32::consts::PI)],
            )],
        );

        let candidate = time_warp_clip_v1(&source, &non_identity_plan(1.0)).unwrap();

        assert_eq!(candidate.tracks[0].times, vec![0.0, 0.5, 1.0]);
        let expected = match sample_track(&source.tracks[0], 0.25) {
            TrackSample::Quat(value) => value,
            TrackSample::Vec3(_) => unreachable!(),
        };
        assert!(quat_values(&candidate.tracks[0])[1].abs_diff_eq(expected, 1.0e-6));
    }
}
