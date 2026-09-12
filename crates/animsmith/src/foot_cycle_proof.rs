//! In-memory serialization, readback, and independent proof for foot-cycle V1.
//!
//! This transaction deliberately stops before evidence encoding, filesystem
//! staging, or publication. Every candidate is counted before any complete GLB
//! is retained; every retained byte vector is then reread and proved as one
//! batch. An error drops the local batch and exposes no partial success.

use std::path::Path;
use std::rc::Rc;

use animsmith_core::metrics::{
    circular_phase_spread, foot_cycle_metrics, loop_continuity_metrics, metric_frame_count,
    root_trajectory_metrics,
};
use animsmith_core::{
    CONTACT_TRANSFORM_RESULT_V1_MAX_CONTROL_POINTS, CollectionLogicalIdV1, ContactEventKindV1,
    ContactFragmentV1, ContactPhaseV1, ContactRoleV1, ContactTimeWarpControlPointV1,
    ContactTransformOperationV1, ContactTransformResultV1, DependencyClosureIdentityV1,
    DependencyClosureV1, DependencyReferenceTargetV1, Document,
    FOOT_CYCLE_PARAMETERIZATION_V1_MAX_ACCUMULATED_YAW_DEG,
    FOOT_CYCLE_PARAMETERIZATION_V1_MAX_HORIZONTAL_DISPLACEMENT_M, FootCycleRootMotionEvidenceV1,
    InputIdentity, Interpolation, MetricGrids, PoseGrid, ResolutionOutcome, Role, Track,
    TrackSample, TrackValues, resolve_configured_roles, sample_track, time_warp_knots_v1,
    validate_document_shape,
};
use animsmith_gltf::write::{
    GlbProjectionPolicyV1, GlbWriteLimits, GlbWritePreflight, preflight_glb_bytes, write_glb_bytes,
};

use super::contact_producer::{
    MAX_METRIC_GRID_WORK, MetricGridWork, checked_metric_grid_work, complete_closure,
    derive_contact_fragment_from_grid,
};
use super::foot_cycle_source_prep::{
    PreparedFootCycleCollectionV1, PreparedFootCycleMemberV1, PreparedFootCycleSourceV1,
};

const MAX_AGGREGATE_CANDIDATE_BYTES: usize = 256 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FootCycleProofKind {
    PreparationBinding,
    ArtifactPreflight,
    ArtifactBudget,
    ArtifactWrite,
    ArtifactReadback,
    ArtifactIdentity,
    ArtifactClosure,
    MetricWork,
    MetricUnavailable,
    ContactDetection,
    ContactTransform,
    Duration,
    ClipMap,
    ContactTopology,
    ContactBoundary,
    GaitAmplitude,
    GaitSpread,
    RootTrajectory,
    LoopContinuity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FootCycleProofError {
    kind: FootCycleProofKind,
}

impl FootCycleProofError {
    const fn new(kind: FootCycleProofKind) -> Self {
        Self { kind }
    }

    #[cfg(test)]
    pub(crate) const fn kind(self) -> FootCycleProofKind {
        self.kind
    }
}

impl std::fmt::Display for FootCycleProofError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "foot-cycle proof failed ({:?})", self.kind)
    }
}

impl std::error::Error for FootCycleProofError {}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FootCycleMemberProofFactsV1 {
    duration_s: f64,
    gait_phase: f64,
    lr_amplitude_m: f64,
    max_contact_boundary_phase_error: f64,
    root_endpoint_displacement_x_m: f64,
    root_endpoint_displacement_z_m: f64,
    root_accumulated_yaw_deg: f64,
    max_loop_position_delta_m: f64,
    max_loop_rotation_delta_deg: f64,
    max_loop_velocity_delta_mps: f64,
    max_loop_angular_velocity_delta_degps: f64,
}

impl FootCycleMemberProofFactsV1 {
    pub(crate) const fn duration_s(&self) -> f64 {
        self.duration_s
    }
    pub(crate) const fn gait_phase(&self) -> f64 {
        self.gait_phase
    }
    pub(crate) const fn lr_amplitude_m(&self) -> f64 {
        self.lr_amplitude_m
    }
    pub(crate) const fn max_contact_boundary_phase_error(&self) -> f64 {
        self.max_contact_boundary_phase_error
    }
    pub(crate) const fn root_endpoint_displacement_x_m(&self) -> f64 {
        self.root_endpoint_displacement_x_m
    }
    pub(crate) const fn root_endpoint_displacement_z_m(&self) -> f64 {
        self.root_endpoint_displacement_z_m
    }
    pub(crate) const fn root_accumulated_yaw_deg(&self) -> f64 {
        self.root_accumulated_yaw_deg
    }
    pub(crate) const fn max_loop_position_delta_m(&self) -> f64 {
        self.max_loop_position_delta_m
    }
    pub(crate) const fn max_loop_rotation_delta_deg(&self) -> f64 {
        self.max_loop_rotation_delta_deg
    }
    pub(crate) const fn max_loop_velocity_delta_mps(&self) -> f64 {
        self.max_loop_velocity_delta_mps
    }
    pub(crate) const fn max_loop_angular_velocity_delta_degps(&self) -> f64 {
        self.max_loop_angular_velocity_delta_degps
    }
}

pub(crate) struct ProvedFootCycleMemberV1 {
    id: CollectionLogicalIdV1,
    artifact_bytes: Vec<u8>,
    artifact: InputIdentity,
    dependency_closure: DependencyClosureV1,
    contact_transform: ContactTransformResultV1,
    independently_detected_contact: ContactFragmentV1,
    facts: FootCycleMemberProofFactsV1,
}

impl ProvedFootCycleMemberV1 {
    pub(crate) fn id(&self) -> &CollectionLogicalIdV1 {
        &self.id
    }
    #[cfg(test)]
    pub(crate) fn artifact_bytes(&self) -> &[u8] {
        &self.artifact_bytes
    }
    pub(crate) fn into_artifact_bytes(self) -> Vec<u8> {
        self.artifact_bytes
    }
    pub(crate) const fn artifact(&self) -> &InputIdentity {
        &self.artifact
    }
    pub(crate) const fn dependency_closure(&self) -> &DependencyClosureV1 {
        &self.dependency_closure
    }
    pub(crate) const fn contact_transform(&self) -> &ContactTransformResultV1 {
        &self.contact_transform
    }
    pub(crate) const fn independently_detected_contact(&self) -> &ContactFragmentV1 {
        &self.independently_detected_contact
    }
    pub(crate) const fn facts(&self) -> &FootCycleMemberProofFactsV1 {
        &self.facts
    }
}

pub(crate) struct ProvedFootCycleCollectionV1 {
    members: Vec<ProvedFootCycleMemberV1>,
    retained_candidate_bytes: usize,
    gait_phase_spread: f64,
    source_metric_pose_cells: usize,
    source_metric_sample_evaluations: usize,
    output_metric_pose_cells: usize,
    output_metric_sample_evaluations: usize,
    metric_pose_cells: usize,
    metric_sample_evaluations: usize,
}

impl ProvedFootCycleCollectionV1 {
    pub(crate) fn members(&self) -> &[ProvedFootCycleMemberV1] {
        &self.members
    }
    pub(crate) fn into_members(self) -> Vec<ProvedFootCycleMemberV1> {
        self.members
    }
    pub(crate) const fn retained_candidate_bytes(&self) -> usize {
        self.retained_candidate_bytes
    }
    pub(crate) const fn gait_phase_spread(&self) -> f64 {
        self.gait_phase_spread
    }
    pub(crate) const fn source_metric_pose_cells(&self) -> usize {
        self.source_metric_pose_cells
    }
    pub(crate) const fn source_metric_sample_evaluations(&self) -> usize {
        self.source_metric_sample_evaluations
    }
    pub(crate) const fn output_metric_pose_cells(&self) -> usize {
        self.output_metric_pose_cells
    }
    pub(crate) const fn output_metric_sample_evaluations(&self) -> usize {
        self.output_metric_sample_evaluations
    }
    pub(crate) const fn metric_pose_cells(&self) -> usize {
        self.metric_pose_cells
    }
    pub(crate) const fn metric_sample_evaluations(&self) -> usize {
        self.metric_sample_evaluations
    }
}

trait FootCycleProofRuntime {
    fn preflight(&mut self, document: &Document) -> Result<GlbWritePreflight, FootCycleProofError>;
    fn write(
        &mut self,
        document: &Document,
        preflight: &GlbWritePreflight,
    ) -> Result<Vec<u8>, FootCycleProofError>;
    fn readback(
        &mut self,
        member_index: usize,
        bytes: &[u8],
    ) -> Result<animsmith_core::LoadedSource, FootCycleProofError>;
    fn build_grid(
        &mut self,
        document: &Document,
        clip_index: usize,
    ) -> Result<Rc<PoseGrid>, FootCycleProofError>;
}

struct ProductionFootCycleProofRuntime;

impl FootCycleProofRuntime for ProductionFootCycleProofRuntime {
    fn preflight(&mut self, document: &Document) -> Result<GlbWritePreflight, FootCycleProofError> {
        preflight_glb_bytes(
            document,
            GlbProjectionPolicyV1::StrictFootCycleV1,
            GlbWriteLimits::FOOT_CYCLE_V1,
        )
        .map_err(|_| FootCycleProofError::new(FootCycleProofKind::ArtifactPreflight))
    }

    fn write(
        &mut self,
        document: &Document,
        preflight: &GlbWritePreflight,
    ) -> Result<Vec<u8>, FootCycleProofError> {
        write_glb_bytes(
            document,
            GlbProjectionPolicyV1::StrictFootCycleV1,
            preflight,
        )
        .map_err(|_| FootCycleProofError::new(FootCycleProofKind::ArtifactWrite))
    }

    fn readback(
        &mut self,
        _member_index: usize,
        bytes: &[u8],
    ) -> Result<animsmith_core::LoadedSource, FootCycleProofError> {
        animsmith_gltf::load_source_bytes(Path::new("artifact.glb"), bytes)
            .map_err(|_| FootCycleProofError::new(FootCycleProofKind::ArtifactReadback))
    }

    fn build_grid(
        &mut self,
        document: &Document,
        clip_index: usize,
    ) -> Result<Rc<PoseGrid>, FootCycleProofError> {
        MetricGrids::new(document)
            .grid(clip_index)
            .ok_or_else(|| FootCycleProofError::new(FootCycleProofKind::MetricUnavailable))
    }
}

struct CandidateDocument {
    document: Document,
    preflight: GlbWritePreflight,
    metric_work: MetricGridWork,
}

struct ReadbackCandidate {
    bytes: Vec<u8>,
    source: animsmith_core::LoadedSource,
    artifact: InputIdentity,
    closure_identity: DependencyClosureIdentityV1,
}

pub(crate) fn serialize_and_prove_foot_cycle_v1(
    prepared: &PreparedFootCycleCollectionV1,
) -> Result<ProvedFootCycleCollectionV1, FootCycleProofError> {
    serialize_and_prove_foot_cycle_v1_with_runtime(prepared, &mut ProductionFootCycleProofRuntime)
}

fn serialize_and_prove_foot_cycle_v1_with_runtime(
    prepared: &PreparedFootCycleCollectionV1,
    runtime: &mut impl FootCycleProofRuntime,
) -> Result<ProvedFootCycleCollectionV1, FootCycleProofError> {
    if prepared.members().len() != prepared.plan().members().len() {
        return Err(FootCycleProofError::new(
            FootCycleProofKind::PreparationBinding,
        ));
    }

    let mut candidate_documents = Vec::with_capacity(prepared.members().len());
    let mut retained_candidate_bytes = 0usize;
    let mut total_metric_work = MetricGridWork {
        pose_cells: prepared.source_metric_pose_cells(),
        sample_evaluations: prepared.source_metric_sample_evaluations(),
    };
    for (member, plan) in prepared.members().iter().zip(prepared.plan().members()) {
        if member.id() != plan.id() {
            return Err(FootCycleProofError::new(
                FootCycleProofKind::PreparationBinding,
            ));
        }
        let source = prepared
            .sources()
            .get(member.source_index())
            .ok_or_else(|| FootCycleProofError::new(FootCycleProofKind::PreparationBinding))?;
        let mut document = source.document().clone();
        let slot = document
            .clips
            .get_mut(member.clip_index())
            .ok_or_else(|| FootCycleProofError::new(FootCycleProofKind::PreparationBinding))?;
        *slot = member.candidate_clip().clone();
        validate_document_shape(&document)
            .map_err(|_| FootCycleProofError::new(FootCycleProofKind::ArtifactPreflight))?;
        let preflight = runtime.preflight(&document)?;
        retained_candidate_bytes =
            add_candidate_bytes(retained_candidate_bytes, preflight.total_bytes())
                .ok_or_else(|| FootCycleProofError::new(FootCycleProofKind::ArtifactBudget))?;
        let metric_work = metric_work(&document, member.clip_index())?;
        total_metric_work = add_metric_work(total_metric_work, metric_work)
            .ok_or_else(|| FootCycleProofError::new(FootCycleProofKind::MetricWork))?;
        candidate_documents.push(CandidateDocument {
            document,
            preflight,
            metric_work,
        });
    }

    let mut serialized = Vec::with_capacity(candidate_documents.len());
    for candidate in &candidate_documents {
        let bytes = runtime.write(&candidate.document, &candidate.preflight)?;
        if bytes.len() != candidate.preflight.total_bytes() {
            return Err(FootCycleProofError::new(FootCycleProofKind::ArtifactWrite));
        }
        serialized.push(bytes);
    }

    let mut readbacks = Vec::with_capacity(serialized.len());
    for (member_index, bytes) in serialized.into_iter().enumerate() {
        let source = runtime.readback(member_index, &bytes)?;
        validate_document_shape(source.document())
            .map_err(|_| FootCycleProofError::new(FootCycleProofKind::ArtifactReadback))?;
        if metric_work(
            source.document(),
            prepared.members()[member_index].clip_index(),
        )? != candidate_documents[member_index].metric_work
        {
            return Err(FootCycleProofError::new(FootCycleProofKind::MetricWork));
        }
        let artifact = InputIdentity::from_bytes(&bytes);
        if source.source_facts().primary_identity() != &artifact {
            return Err(FootCycleProofError::new(
                FootCycleProofKind::ArtifactIdentity,
            ));
        }
        let closure = source.dependency_closure();
        let closure_identity = complete_closure(closure, &artifact)
            .map_err(|_| FootCycleProofError::new(FootCycleProofKind::ArtifactClosure))?;
        if !is_self_contained_closure(closure) {
            return Err(FootCycleProofError::new(
                FootCycleProofKind::ArtifactClosure,
            ));
        }
        readbacks.push(ReadbackCandidate {
            bytes,
            source,
            artifact,
            closure_identity,
        });
    }

    let mut proved = Vec::with_capacity(readbacks.len());
    let mut gait_phases = Vec::with_capacity(readbacks.len());
    for (member_index, readback) in readbacks.into_iter().enumerate() {
        let member = &prepared.members()[member_index];
        let plan = &prepared.plan().members()[member_index];
        let source = prepared
            .sources()
            .get(member.source_index())
            .ok_or_else(|| FootCycleProofError::new(FootCycleProofKind::PreparationBinding))?;
        let document = readback.source.document();
        let clip = document
            .clips
            .get(member.clip_index())
            .ok_or_else(|| FootCycleProofError::new(FootCycleProofKind::PreparationBinding))?;
        let grid = runtime.build_grid(document, member.clip_index())?;
        let detected = derive_contact_fragment_from_grid(
            document,
            source.config(),
            member.clip_index(),
            &readback.artifact,
            readback.closure_identity.clone(),
            member.contact_transform().input_fragment().clip().clone(),
            &grid,
        )
        .map_err(|_| FootCycleProofError::new(FootCycleProofKind::ContactDetection))?;
        let transformed = member
            .contact_transform()
            .transform_after_serialization(
                readback.artifact.clone(),
                readback.source.dependency_closure().clone(),
                detected.producer().clone(),
            )
            .map_err(|_| FootCycleProofError::new(FootCycleProofKind::ContactTransform))?;
        let expected_contact = transformed
            .output()
            .ok_or_else(|| FootCycleProofError::new(FootCycleProofKind::ContactTransform))?
            .contact_fragment();
        let facts = prove_member(
            source,
            member,
            plan.root_motion(),
            plan.operation(),
            prepared.plan().proof(),
            document,
            clip,
            &grid,
            expected_contact,
            &detected,
        )?;
        gait_phases.push(facts.gait_phase);
        proved.push(ProvedFootCycleMemberV1 {
            id: member.id().clone(),
            artifact_bytes: readback.bytes,
            artifact: readback.artifact,
            dependency_closure: readback.source.dependency_closure().clone(),
            contact_transform: transformed,
            independently_detected_contact: detected,
            facts,
        });
    }
    let gait_phase_spread = prove_gait_spread(
        &gait_phases,
        prepared.plan().proof().max_gait_phase_spread(),
    )?;

    let output_metric_pose_cells = total_metric_work
        .pose_cells
        .checked_sub(prepared.source_metric_pose_cells())
        .ok_or_else(|| FootCycleProofError::new(FootCycleProofKind::MetricWork))?;
    let output_metric_sample_evaluations = total_metric_work
        .sample_evaluations
        .checked_sub(prepared.source_metric_sample_evaluations())
        .ok_or_else(|| FootCycleProofError::new(FootCycleProofKind::MetricWork))?;

    Ok(ProvedFootCycleCollectionV1 {
        members: proved,
        retained_candidate_bytes,
        gait_phase_spread,
        source_metric_pose_cells: prepared.source_metric_pose_cells(),
        source_metric_sample_evaluations: prepared.source_metric_sample_evaluations(),
        output_metric_pose_cells,
        output_metric_sample_evaluations,
        metric_pose_cells: total_metric_work.pose_cells,
        metric_sample_evaluations: total_metric_work.sample_evaluations,
    })
}

#[allow(clippy::too_many_arguments)]
fn prove_member(
    source: &PreparedFootCycleSourceV1,
    member: &PreparedFootCycleMemberV1,
    source_root: &FootCycleRootMotionEvidenceV1,
    operation: &ContactTransformOperationV1,
    policy: &animsmith_core::FootCycleProofPolicyV1,
    output_document: &Document,
    clip: &animsmith_core::Clip,
    grid: &PoseGrid,
    expected_contact: &ContactFragmentV1,
    detected_contact: &ContactFragmentV1,
) -> Result<FootCycleMemberProofFactsV1, FootCycleProofError> {
    let source_duration = source
        .document()
        .clips
        .get(member.clip_index())
        .map(|clip| clip.duration_s)
        .ok_or_else(|| FootCycleProofError::new(FootCycleProofKind::Duration))?;
    prove_duration(source_duration, clip, expected_contact, detected_contact)?;
    let source_clip = source
        .document()
        .clips
        .get(member.clip_index())
        .ok_or_else(|| FootCycleProofError::new(FootCycleProofKind::ClipMap))?;
    prove_clip_map(source_clip, clip, operation)?;

    let max_contact_boundary_phase_error = prove_contact_boundaries(
        expected_contact,
        detected_contact,
        policy.max_contact_boundary_phase_error(),
    )?;
    let roles = resolve_configured_roles(&output_document.skeleton, &source.config().rig);
    if matches!(
        roles.outcome(),
        ResolutionOutcome::AmbiguousExactMatch
            | ResolutionOutcome::AmbiguousFoldedMatch
            | ResolutionOutcome::RoleCollision
            | ResolutionOutcome::AmbiguousProfile
    ) {
        return Err(FootCycleProofError::new(
            FootCycleProofKind::MetricUnavailable,
        ));
    }
    let (gait_phase, lr_amplitude_m) = prove_gait(
        grid,
        &roles,
        source.config().loop_seam_min_stride_step_m(),
        policy.min_lr_amplitude_m(),
    )?;

    let root_bone = roles
        .get(Role::Root)
        .or_else(|| roles.get(Role::Hips))
        .ok_or_else(|| FootCycleProofError::new(FootCycleProofKind::RootTrajectory))?;
    let root = root_trajectory_metrics(grid, root_bone)
        .ok_or_else(|| FootCycleProofError::new(FootCycleProofKind::RootTrajectory))?;
    let translation = root
        .translation
        .ok_or_else(|| FootCycleProofError::new(FootCycleProofKind::RootTrajectory))?;
    let yaw = root
        .yaw
        .ok_or_else(|| FootCycleProofError::new(FootCycleProofKind::RootTrajectory))?;
    let FootCycleRootMotionEvidenceV1::Measured {
        endpoint_displacement_x_m: source_x,
        endpoint_displacement_z_m: source_z,
        accumulated_yaw_deg: source_yaw,
        ..
    } = source_root
    else {
        return Err(FootCycleProofError::new(FootCycleProofKind::RootTrajectory));
    };
    let output_x = translation.horizontal_displacement_x_m;
    let output_z = translation.horizontal_displacement_z_m;
    let output_yaw = yaw.unwrapped_yaw_deg;
    prove_root_values(
        [*source_x, *source_z, *source_yaw],
        [output_x, output_z, output_yaw],
    )?;

    let expectations = source.config().expectations_for(&clip.name);
    if expectations.looping != Some(true) {
        return Err(FootCycleProofError::new(FootCycleProofKind::LoopContinuity));
    }
    let tolerances = source.config().loop_continuity_tolerances(&clip.name);
    let loop_rows = loop_continuity_metrics(grid)
        .ok_or_else(|| FootCycleProofError::new(FootCycleProofKind::LoopContinuity))?;
    let mut loop_maxima = [0.0f64; 4];
    for row in loop_rows {
        let row =
            row.ok_or_else(|| FootCycleProofError::new(FootCycleProofKind::LoopContinuity))?;
        loop_maxima[0] = loop_maxima[0].max(row.position_delta_m);
        loop_maxima[1] = loop_maxima[1].max(row.rotation_delta_deg);
        loop_maxima[2] = loop_maxima[2].max(row.seam_velocity_delta_mps);
        loop_maxima[3] = loop_maxima[3].max(row.seam_angular_velocity_delta_degps);
    }
    if exceeds_f32_cap(loop_maxima[0], tolerances.max_position_delta_m())
        || exceeds_f32_cap(loop_maxima[1], tolerances.max_rotation_delta_deg())
        || exceeds_f32_cap(loop_maxima[2], tolerances.max_velocity_delta_mps())
        || exceeds_f32_cap(
            loop_maxima[3],
            tolerances.max_angular_velocity_delta_degps(),
        )
    {
        return Err(FootCycleProofError::new(FootCycleProofKind::LoopContinuity));
    }

    Ok(FootCycleMemberProofFactsV1 {
        duration_s: clip.duration_s,
        gait_phase,
        lr_amplitude_m,
        max_contact_boundary_phase_error,
        root_endpoint_displacement_x_m: output_x,
        root_endpoint_displacement_z_m: output_z,
        root_accumulated_yaw_deg: output_yaw,
        max_loop_position_delta_m: loop_maxima[0],
        max_loop_rotation_delta_deg: loop_maxima[1],
        max_loop_velocity_delta_mps: loop_maxima[2],
        max_loop_angular_velocity_delta_degps: loop_maxima[3],
    })
}

fn prove_duration(
    source_duration: f64,
    output: &animsmith_core::Clip,
    expected_contact: &ContactFragmentV1,
    detected_contact: &ContactFragmentV1,
) -> Result<(), FootCycleProofError> {
    if !source_duration.is_finite()
        || source_duration <= 0.0
        || output.duration_s != source_duration
        || expected_contact.duration_s() != source_duration
        || detected_contact.duration_s() != source_duration
    {
        return Err(FootCycleProofError::new(FootCycleProofKind::Duration));
    }
    Ok(())
}

fn prove_gait(
    grid: &PoseGrid,
    roles: &animsmith_core::ResolvedRoles,
    min_stride_step_m: f64,
    min_lr_amplitude_m: f64,
) -> Result<(f64, f64), FootCycleProofError> {
    let gait = foot_cycle_metrics(grid, roles, min_stride_step_m)
        .ok_or_else(|| FootCycleProofError::new(FootCycleProofKind::MetricUnavailable))?;
    let gait_phase = gait
        .gait_phase
        .filter(|phase| phase.is_finite())
        .ok_or_else(|| FootCycleProofError::new(FootCycleProofKind::GaitAmplitude))?;
    if !gait.lr_amplitude_m.is_finite() || gait.lr_amplitude_m < min_lr_amplitude_m {
        return Err(FootCycleProofError::new(FootCycleProofKind::GaitAmplitude));
    }
    Ok((gait_phase, gait.lr_amplitude_m))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContactSide {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContactEdge {
    Onset,
    Release,
}

#[derive(Debug, Clone, Copy)]
struct ContactBoundary {
    phase: f64,
    side: ContactSide,
    edge: ContactEdge,
}

#[derive(Debug, Clone, Copy)]
struct ContactWindow {
    side: ContactSide,
    start: f64,
    end: f64,
}

#[derive(Debug, Clone, Copy)]
struct LogicalContactWindow {
    side: ContactSide,
    onset: f64,
    release: f64,
}

fn prove_contact_boundaries(
    expected: &ContactFragmentV1,
    detected: &ContactFragmentV1,
    max_error: f64,
) -> Result<f64, FootCycleProofError> {
    let expected = contact_boundaries(expected)?;
    let detected = contact_boundaries(detected)?;
    if expected.len() != detected.len() {
        return Err(FootCycleProofError::new(
            FootCycleProofKind::ContactTopology,
        ));
    }
    if expected.iter().zip(&detected).any(|(expected, detected)| {
        expected.side != detected.side || expected.edge != detected.edge
    }) || expected
        .iter()
        .zip(expected.iter().cycle().skip(1))
        .zip(detected.iter().zip(detected.iter().cycle().skip(1)))
        .take(expected.len())
        .any(|((expected, expected_next), (detected, detected_next))| {
            (expected.phase == expected_next.phase) != (detected.phase == detected_next.phase)
        })
    {
        return Err(FootCycleProofError::new(
            FootCycleProofKind::ContactTopology,
        ));
    }
    let mut observed = 0.0f64;
    for (expected, detected) in expected.iter().zip(&detected) {
        let error = circular_phase_distance(expected.phase, detected.phase);
        observed = observed.max(error);
        if !error.is_finite() || error > max_error {
            return Err(FootCycleProofError::new(
                FootCycleProofKind::ContactBoundary,
            ));
        }
    }
    Ok(observed)
}

fn prove_clip_map(
    source: &animsmith_core::Clip,
    output: &animsmith_core::Clip,
    operation: &ContactTransformOperationV1,
) -> Result<(), FootCycleProofError> {
    let ContactTransformOperationV1::TimeWarp {
        version: 1,
        output_duration_s,
        control_points,
    } = operation
    else {
        return Err(FootCycleProofError::new(FootCycleProofKind::ClipMap));
    };
    if *output_duration_s != source.duration_s
        || output.duration_s != source.duration_s
        || output.name != source.name
        || output.tracks.len() != source.tracks.len()
        || !(2..=CONTACT_TRANSFORM_RESULT_V1_MAX_CONTROL_POINTS).contains(&control_points.len())
        || control_points
            .first()
            .is_none_or(|point| point.input_time() != 0.0 || point.output_time() != 0.0)
        || control_points
            .last()
            .is_none_or(|point| point.input_time() != 1.0 || point.output_time() != 1.0)
        || control_points.iter().any(|point| {
            !point.input_time().is_finite()
                || !point.output_time().is_finite()
                || !(0.0..=1.0).contains(&point.input_time())
                || !(0.0..=1.0).contains(&point.output_time())
        })
        || control_points.windows(2).any(|pair| {
            pair[0].input_time() >= pair[1].input_time()
                || pair[0].output_time() >= pair[1].output_time()
        })
    {
        return Err(FootCycleProofError::new(FootCycleProofKind::ClipMap));
    }
    let duration = source.duration_s as f32;
    if !duration.is_finite() || duration <= 0.0 {
        return Err(FootCycleProofError::new(FootCycleProofKind::ClipMap));
    }
    let identity = control_points
        .iter()
        .all(|point| point.input_time() == point.output_time());
    for (source, output) in source.tracks.iter().zip(&output.tracks) {
        if source.bone != output.bone
            || source.property != output.property
            || source.interpolation != output.interpolation
        {
            return Err(FootCycleProofError::new(FootCycleProofKind::ClipMap));
        }
        if source.interpolation == Interpolation::CubicSpline
            && !is_admissible_constant_cubic(source)
        {
            return Err(FootCycleProofError::new(FootCycleProofKind::ClipMap));
        }
        if identity || source.interpolation == Interpolation::CubicSpline {
            if !tracks_equal_bits(source, output) {
                return Err(FootCycleProofError::new(FootCycleProofKind::ClipMap));
            }
            continue;
        }
        // Coincidence between a control point and an authored key is the
        // producer's contract, not this proof's opinion, so the expectation is
        // rebuilt from the same emitted-binary32 question the builder asks:
        // see `FootCycleClipWarpKnotV1`. What the proof keeps to itself is the
        // expectation, which it derives from the source clip and the operation
        // alone and then compares bit for bit against the produced clip.
        let mut knots = time_warp_knots_v1(control_points, duration, source)
            .collect::<Vec<_>>()
            .into_iter()
            .peekable();
        let mut expected = Vec::with_capacity(source.times.len() + knots.len());
        for key in 0..source.times.len() {
            let source_time = source.times[key];
            while knots.peek().is_some_and(|knot| {
                knot.source_time() < source_time && !knot.coincides_with(source_time)
            }) {
                let knot = knots.next().expect("peeked knot");
                expected.push((knot.output_time(), sample_track(source, knot.source_time())));
            }
            let output_time = match knots.next_if(|knot| knot.coincides_with(source_time)) {
                Some(knot) => knot.output_time(),
                None => map_source_time(source_time, duration, control_points)?,
            };
            expected.push((output_time, key_sample(source, key)?));
        }
        for knot in knots {
            expected.push((knot.output_time(), sample_track(source, knot.source_time())));
        }
        if expected.len() != output.times.len()
            || expected.windows(2).any(|pair| pair[0].0 >= pair[1].0)
            || expected
                .iter()
                .zip(&output.times)
                .any(|((expected, _), actual)| expected.to_bits() != actual.to_bits())
            || expected.iter().enumerate().any(|(key, (_, expected))| {
                match key_sample(output, key) {
                    Ok(actual) => !samples_equal_bits(*expected, actual),
                    Err(_) => true,
                }
            })
        {
            return Err(FootCycleProofError::new(FootCycleProofKind::ClipMap));
        }
    }
    Ok(())
}

fn map_source_time(
    source_time: f32,
    duration: f32,
    points: &[ContactTimeWarpControlPointV1],
) -> Result<f32, FootCycleProofError> {
    if points.len() < 2 {
        return Err(FootCycleProofError::new(FootCycleProofKind::ClipMap));
    }
    let normalized = f64::from(source_time) / f64::from(duration);
    let upper = points.partition_point(|point| point.input_time() <= normalized);
    let right = upper.clamp(1, points.len() - 1);
    let left = right - 1;
    let x0 = points[left].input_time();
    let x1 = points[right].input_time();
    let y0 = points[left].output_time();
    let y1 = points[right].output_time();
    if !(x0 < x1 && y0 < y1) {
        return Err(FootCycleProofError::new(FootCycleProofKind::ClipMap));
    }
    let fraction = (normalized - x0) / (x1 - x0);
    let output = ((y0 + fraction * (y1 - y0)) * f64::from(duration)) as f32;
    output
        .is_finite()
        .then_some(output)
        .ok_or_else(|| FootCycleProofError::new(FootCycleProofKind::ClipMap))
}

fn key_sample(track: &Track, key: usize) -> Result<TrackSample, FootCycleProofError> {
    match &track.values {
        TrackValues::Vec3s(values) => values
            .get(track.value_index(key))
            .copied()
            .map(TrackSample::Vec3),
        TrackValues::Quats(values) => values
            .get(track.value_index(key))
            .copied()
            .map(TrackSample::Quat),
    }
    .ok_or_else(|| FootCycleProofError::new(FootCycleProofKind::ClipMap))
}

fn samples_equal_bits(left: TrackSample, right: TrackSample) -> bool {
    match (left, right) {
        (TrackSample::Vec3(left), TrackSample::Vec3(right)) => left
            .to_array()
            .into_iter()
            .zip(right.to_array())
            .all(|(left, right)| left.to_bits() == right.to_bits()),
        (TrackSample::Quat(left), TrackSample::Quat(right)) => left
            .to_array()
            .into_iter()
            .zip(right.to_array())
            .all(|(left, right)| left.to_bits() == right.to_bits()),
        _ => false,
    }
}

fn tracks_equal_bits(left: &Track, right: &Track) -> bool {
    left.bone == right.bone
        && left.property == right.property
        && left.interpolation == right.interpolation
        && left.times.len() == right.times.len()
        && left
            .times
            .iter()
            .zip(&right.times)
            .all(|(left, right)| left.to_bits() == right.to_bits())
        && left.values.len() == right.values.len()
        && match (&left.values, &right.values) {
            (TrackValues::Vec3s(left), TrackValues::Vec3s(right)) => {
                left.iter().zip(right).all(|(left, right)| {
                    samples_equal_bits(TrackSample::Vec3(*left), TrackSample::Vec3(*right))
                })
            }
            (TrackValues::Quats(left), TrackValues::Quats(right)) => {
                left.iter().zip(right).all(|(left, right)| {
                    samples_equal_bits(TrackSample::Quat(*left), TrackSample::Quat(*right))
                })
            }
            _ => false,
        }
}

#[cfg(test)]
fn clips_equal_bits(left: &animsmith_core::Clip, right: &animsmith_core::Clip) -> bool {
    left.name == right.name
        && left.duration_s.to_bits() == right.duration_s.to_bits()
        && left.tracks.len() == right.tracks.len()
        && left
            .tracks
            .iter()
            .zip(&right.tracks)
            .all(|(left, right)| tracks_equal_bits(left, right))
}

fn is_admissible_constant_cubic(track: &Track) -> bool {
    if track.interpolation != Interpolation::CubicSpline || track.times.is_empty() {
        return false;
    }
    if track.times.len() == 1 {
        return true;
    }
    match &track.values {
        TrackValues::Vec3s(values) if values.len() == 3 * track.times.len() => {
            let reference = TrackSample::Vec3(values[1]);
            (0..track.times.len()).all(|key| {
                samples_equal_bits(TrackSample::Vec3(values[3 * key + 1]), reference)
                    && values[3 * key]
                        .to_array()
                        .into_iter()
                        .all(|component| component == 0.0)
                    && values[3 * key + 2]
                        .to_array()
                        .into_iter()
                        .all(|component| component == 0.0)
            })
        }
        TrackValues::Quats(values) if values.len() == 3 * track.times.len() => {
            let reference = TrackSample::Quat(values[1]);
            (0..track.times.len()).all(|key| {
                samples_equal_bits(TrackSample::Quat(values[3 * key + 1]), reference)
                    && values[3 * key]
                        .to_array()
                        .into_iter()
                        .all(|component| component == 0.0)
                    && values[3 * key + 2]
                        .to_array()
                        .into_iter()
                        .all(|component| component == 0.0)
            })
        }
        _ => false,
    }
}

fn contact_boundaries(
    fragment: &ContactFragmentV1,
) -> Result<Vec<ContactBoundary>, FootCycleProofError> {
    let mut windows = [Vec::new(), Vec::new()];
    let mut markers = [Vec::new(), Vec::new()];
    for event in fragment.events() {
        let side = match event.role() {
            ContactRoleV1::LeftFoot | ContactRoleV1::LeftToe => ContactSide::Left,
            ContactRoleV1::RightFoot | ContactRoleV1::RightToe => ContactSide::Right,
            _ => {
                return Err(FootCycleProofError::new(
                    FootCycleProofKind::ContactTopology,
                ));
            }
        };
        match (event.phase(), event.kind()) {
            (ContactPhaseV1::Begin, ContactEventKindV1::Window(window))
                if window.start() < window.end() =>
            {
                windows[contact_side_index(side)].push(ContactWindow {
                    side,
                    start: window.start(),
                    end: window.end(),
                });
            }
            (ContactPhaseV1::Marker, ContactEventKindV1::Point(phase)) => {
                markers[contact_side_index(side)].push(phase);
            }
            _ => {
                return Err(FootCycleProofError::new(
                    FootCycleProofKind::ContactTopology,
                ));
            }
        }
    }

    let mut logical = Vec::new();
    for (side_windows, side_markers) in windows.iter_mut().zip(&mut markers) {
        side_windows.sort_by(|left, right| left.start.total_cmp(&right.start));
        side_markers.sort_by(|left, right| left.total_cmp(right));
        if side_windows.is_empty()
            || side_windows.len() != side_markers.len()
            || side_windows
                .windows(2)
                .any(|pair| pair[0].end >= pair[1].start)
            || side_windows
                .iter()
                .zip(side_markers)
                .any(|(window, marker)| *marker < window.start || *marker > window.end)
        {
            return Err(FootCycleProofError::new(
                FootCycleProofKind::ContactTopology,
            ));
        }
        let first = side_windows[0];
        let last = side_windows[side_windows.len() - 1];
        if first.start == 0.0 && last.end == 1.0 {
            if side_windows.len() == 1 {
                return Err(FootCycleProofError::new(
                    FootCycleProofKind::ContactTopology,
                ));
            }
            logical.push(LogicalContactWindow {
                side: first.side,
                onset: last.start,
                release: first.end,
            });
            logical.extend(
                side_windows[1..side_windows.len() - 1]
                    .iter()
                    .map(|window| LogicalContactWindow {
                        side: window.side,
                        onset: window.start,
                        release: window.end,
                    }),
            );
        } else {
            logical.extend(side_windows.iter().map(|window| LogicalContactWindow {
                side: window.side,
                onset: window.start,
                release: window.end,
            }));
        }
    }

    let left_count = logical
        .iter()
        .filter(|window| window.side == ContactSide::Left)
        .count();
    let right_count = logical.len() - left_count;
    logical.sort_by(|left, right| {
        left.onset
            .total_cmp(&right.onset)
            .then_with(|| contact_side_index(left.side).cmp(&contact_side_index(right.side)))
    });
    if left_count == 0
        || left_count != right_count
        || logical
            .iter()
            .zip(logical.iter().cycle().skip(1))
            .take(logical.len())
            .any(|(left, right)| left.side == right.side)
    {
        return Err(FootCycleProofError::new(
            FootCycleProofKind::ContactTopology,
        ));
    }

    let mut boundaries = Vec::with_capacity(logical.len() * 2);
    for window in logical {
        boundaries.push(ContactBoundary {
            phase: window.onset,
            side: window.side,
            edge: ContactEdge::Onset,
        });
        boundaries.push(ContactBoundary {
            phase: window.release,
            side: window.side,
            edge: ContactEdge::Release,
        });
    }
    boundaries.sort_by(|left, right| {
        left.phase.total_cmp(&right.phase).then_with(|| {
            contact_boundary_key(left.side, left.edge)
                .cmp(&contact_boundary_key(right.side, right.edge))
        })
    });
    Ok(boundaries)
}

const fn contact_side_index(side: ContactSide) -> usize {
    match side {
        ContactSide::Left => 0,
        ContactSide::Right => 1,
    }
}

const fn contact_boundary_key(side: ContactSide, edge: ContactEdge) -> (usize, usize) {
    (
        contact_side_index(side),
        match edge {
            ContactEdge::Onset => 0,
            ContactEdge::Release => 1,
        },
    )
}

fn metric_work(
    document: &Document,
    clip_index: usize,
) -> Result<MetricGridWork, FootCycleProofError> {
    let clip = document
        .clips
        .get(clip_index)
        .ok_or_else(|| FootCycleProofError::new(FootCycleProofKind::MetricWork))?;
    let frame_count = metric_frame_count(clip)
        .ok_or_else(|| FootCycleProofError::new(FootCycleProofKind::MetricWork))?;
    checked_metric_grid_work(
        frame_count,
        document.skeleton.bones.len(),
        clip.tracks.len(),
    )
    .ok_or_else(|| FootCycleProofError::new(FootCycleProofKind::MetricWork))
}

fn add_candidate_bytes(total: usize, next: usize) -> Option<usize> {
    total
        .checked_add(next)
        .filter(|value| *value <= MAX_AGGREGATE_CANDIDATE_BYTES)
}

fn is_self_contained_closure(closure: &DependencyClosureV1) -> bool {
    closure.external_resources().is_empty()
        && closure
            .references()
            .iter()
            .all(|reference| reference.target() == &DependencyReferenceTargetV1::Primary)
}

fn add_metric_work(total: MetricGridWork, next: MetricGridWork) -> Option<MetricGridWork> {
    Some(MetricGridWork {
        pose_cells: total
            .pose_cells
            .checked_add(next.pose_cells)
            .filter(|value| *value <= MAX_METRIC_GRID_WORK)?,
        sample_evaluations: total
            .sample_evaluations
            .checked_add(next.sample_evaluations)
            .filter(|value| *value <= MAX_METRIC_GRID_WORK)?,
    })
}

fn circular_phase_distance(left: f64, right: f64) -> f64 {
    let difference = (left - right).abs().rem_euclid(1.0);
    difference.min(1.0 - difference)
}

fn prove_gait_spread(phases: &[f64], max_spread: f64) -> Result<f64, FootCycleProofError> {
    let spread = circular_phase_spread(phases);
    if !spread.is_finite() || spread > max_spread {
        return Err(FootCycleProofError::new(FootCycleProofKind::GaitSpread));
    }
    Ok(spread)
}

fn signed_angle_distance(left: f64, right: f64) -> f64 {
    (left - right + 180.0).rem_euclid(360.0) - 180.0
}

fn prove_root_values(source: [f64; 3], output: [f64; 3]) -> Result<(), FootCycleProofError> {
    let [source_x, source_z, source_yaw] = source;
    let [output_x, output_z, output_yaw] = output;
    if source
        .into_iter()
        .chain(output)
        .any(|value| !value.is_finite())
        || output_x.hypot(output_z) > FOOT_CYCLE_PARAMETERIZATION_V1_MAX_HORIZONTAL_DISPLACEMENT_M
        || output_yaw.abs() > FOOT_CYCLE_PARAMETERIZATION_V1_MAX_ACCUMULATED_YAW_DEG
        || source_x.hypot(source_z) > FOOT_CYCLE_PARAMETERIZATION_V1_MAX_HORIZONTAL_DISPLACEMENT_M
        || source_yaw.abs() > FOOT_CYCLE_PARAMETERIZATION_V1_MAX_ACCUMULATED_YAW_DEG
        || (output_x - source_x).hypot(output_z - source_z)
            > FOOT_CYCLE_PARAMETERIZATION_V1_MAX_HORIZONTAL_DISPLACEMENT_M
        || signed_angle_distance(output_yaw, source_yaw).abs()
            > FOOT_CYCLE_PARAMETERIZATION_V1_MAX_ACCUMULATED_YAW_DEG
    {
        return Err(FootCycleProofError::new(FootCycleProofKind::RootTrajectory));
    }
    Ok(())
}

fn exceeds_f32_cap(measured: f64, cap: f64) -> bool {
    (measured as f32) > (cap as f32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use animsmith_core::{
        ContactClipReferenceV1, ContactEventV1, ContactEventWindowV1, ContactProducerV1,
        DependencyClosureBuilderV1, DependencyResourceKeyV1, ResourceKeySyntaxV1,
        SourceResourceKindV1, SourceResourceLocatorV1, SourceSetCoverageV1,
    };

    fn contact_fragment(windows: &[(ContactRoleV1, f64, f64)]) -> ContactFragmentV1 {
        let artifact = InputIdentity::from_bytes(b"contact-proof-fixture");
        let closure =
            DependencyClosureBuilderV1::new(artifact.clone(), SourceSetCoverageV1::complete(), 0)
                .finish()
                .unwrap()
                .identity()
                .unwrap()
                .clone();
        let mut events = Vec::new();
        for (index, &(role, start, end)) in windows.iter().enumerate() {
            events.push(
                ContactEventV1::window(
                    format!("support/{index}"),
                    role,
                    ContactPhaseV1::Begin,
                    ContactEventWindowV1::new(start, end).unwrap(),
                    None,
                )
                .unwrap(),
            );
            events.push(
                ContactEventV1::point(
                    format!("marker/{index}"),
                    role,
                    ContactPhaseV1::Marker,
                    (start + end) / 2.0,
                    None,
                )
                .unwrap(),
            );
        }
        ContactFragmentV1::new(
            ContactProducerV1::new("animsmith", "test").unwrap(),
            artifact,
            closure,
            ContactClipReferenceV1::document("walk").unwrap(),
            1.0,
            events,
            Vec::new(),
        )
        .unwrap()
    }

    #[derive(Default)]
    struct ObservedRuntime {
        preflights: usize,
        writes: usize,
        readbacks: usize,
        fail_preflight_at: Option<usize>,
        fail_write_at: Option<usize>,
        mutate_readback_at: Option<usize>,
        grid_mutation: Option<(usize, GridMutation)>,
        grids_built: usize,
    }

    #[derive(Clone, Copy)]
    enum GridMutation {
        ContactTopology,
        LowGaitAmplitude,
        RootTrajectory,
        LoopContinuity,
    }

    impl FootCycleProofRuntime for ObservedRuntime {
        fn preflight(
            &mut self,
            document: &Document,
        ) -> Result<GlbWritePreflight, FootCycleProofError> {
            let index = self.preflights;
            self.preflights += 1;
            if self.fail_preflight_at == Some(index) {
                return Err(FootCycleProofError::new(
                    FootCycleProofKind::ArtifactPreflight,
                ));
            }
            ProductionFootCycleProofRuntime.preflight(document)
        }

        fn write(
            &mut self,
            document: &Document,
            preflight: &GlbWritePreflight,
        ) -> Result<Vec<u8>, FootCycleProofError> {
            let index = self.writes;
            self.writes += 1;
            if self.fail_write_at == Some(index) {
                return Err(FootCycleProofError::new(FootCycleProofKind::ArtifactWrite));
            }
            ProductionFootCycleProofRuntime.write(document, preflight)
        }

        fn readback(
            &mut self,
            member_index: usize,
            bytes: &[u8],
        ) -> Result<animsmith_core::LoadedSource, FootCycleProofError> {
            self.readbacks += 1;
            if self.mutate_readback_at == Some(member_index) {
                let mut mutated = bytes.to_vec();
                let offset = mutated
                    .windows(b"animsmith".len())
                    .position(|window| window == b"animsmith")
                    .expect("strict GLB records the generator");
                mutated[offset] = b'A';
                return ProductionFootCycleProofRuntime.readback(member_index, &mutated);
            }
            ProductionFootCycleProofRuntime.readback(member_index, bytes)
        }

        fn build_grid(
            &mut self,
            document: &Document,
            clip_index: usize,
        ) -> Result<Rc<PoseGrid>, FootCycleProofError> {
            let index = self.grids_built;
            self.grids_built += 1;
            let Some((target, mutation)) = self.grid_mutation else {
                return ProductionFootCycleProofRuntime.build_grid(document, clip_index);
            };
            if target != index {
                return ProductionFootCycleProofRuntime.build_grid(document, clip_index);
            }
            let mut mutated = document.clone();
            match mutation {
                GridMutation::ContactTopology => {
                    for track in &mut mutated.clips[clip_index].tracks {
                        if track.bone >= 2
                            && let TrackValues::Vec3s(values) = &mut track.values
                        {
                            for value in values {
                                value.y = 1.0;
                            }
                        }
                    }
                }
                GridMutation::LowGaitAmplitude => {
                    for track in &mut mutated.clips[clip_index].tracks {
                        if track.bone >= 2
                            && let TrackValues::Vec3s(values) = &mut track.values
                        {
                            for value in values {
                                value.y *= 0.2;
                            }
                        }
                    }
                }
                GridMutation::RootTrajectory => {
                    let root = mutated.clips[clip_index]
                        .tracks
                        .iter_mut()
                        .find(|track| track.bone == 0)
                        .expect("fixture root track");
                    let key_count = root.times.len();
                    let TrackValues::Vec3s(values) = &mut root.values else {
                        panic!("fixture root translation");
                    };
                    for (key, value) in values.iter_mut().enumerate() {
                        value.x += 0.02 * key as f32 / (key_count - 1) as f32;
                    }
                }
                GridMutation::LoopContinuity => {
                    let foot = mutated.clips[clip_index]
                        .tracks
                        .iter_mut()
                        .find(|track| track.bone >= 2)
                        .expect("fixture foot track");
                    let TrackValues::Vec3s(values) = &mut foot.values else {
                        panic!("fixture foot translation");
                    };
                    values.last_mut().expect("fixture endpoint").x += 0.1;
                }
            }
            ProductionFootCycleProofRuntime.build_grid(&mutated, clip_index)
        }
    }

    #[test]
    fn aggregate_candidate_and_metric_bounds_are_inclusive_and_checked() {
        assert_eq!(
            MAX_AGGREGATE_CANDIDATE_BYTES,
            GlbWriteLimits::FOOT_CYCLE_V1.max_total_bytes
        );
        assert_eq!(
            add_candidate_bytes(MAX_AGGREGATE_CANDIDATE_BYTES - 1, 1),
            Some(MAX_AGGREGATE_CANDIDATE_BYTES)
        );
        assert_eq!(add_candidate_bytes(MAX_AGGREGATE_CANDIDATE_BYTES, 1), None);
        assert_eq!(add_candidate_bytes(usize::MAX, 1), None);
        assert_eq!(
            add_metric_work(
                MetricGridWork {
                    pose_cells: MAX_METRIC_GRID_WORK,
                    sample_evaluations: MAX_METRIC_GRID_WORK,
                },
                MetricGridWork {
                    pose_cells: 0,
                    sample_evaluations: 0,
                },
            ),
            Some(MetricGridWork {
                pose_cells: MAX_METRIC_GRID_WORK,
                sample_evaluations: MAX_METRIC_GRID_WORK,
            })
        );
        assert!(
            add_metric_work(
                MetricGridWork {
                    pose_cells: MAX_METRIC_GRID_WORK,
                    sample_evaluations: 0,
                },
                MetricGridWork {
                    pose_cells: 1,
                    sample_evaluations: 0,
                },
            )
            .is_none()
        );
        assert!(
            add_metric_work(
                MetricGridWork {
                    pose_cells: usize::MAX,
                    sample_evaluations: 0,
                },
                MetricGridWork {
                    pose_cells: 1,
                    sample_evaluations: 0,
                },
            )
            .is_none()
        );
    }

    #[test]
    fn self_contained_closure_allows_primary_references_and_refuses_external_resources() {
        let artifact = InputIdentity::from_bytes(b"closure-proof");
        let mut primary_builder =
            DependencyClosureBuilderV1::new(artifact.clone(), SourceSetCoverageV1::complete(), 1);
        assert!(primary_builder.begin_reference(8, 0));
        primary_builder
            .push_primary(0, SourceResourceKindV1::Buffer, 0)
            .unwrap();
        let primary = primary_builder.finish().unwrap();
        assert!(is_self_contained_closure(&primary));

        let SourceResourceLocatorV1::Relative(locator) =
            SourceResourceLocatorV1::classify("external.bin")
        else {
            panic!("safe relative fixture");
        };
        let key =
            DependencyResourceKeyV1::from_relative(&locator, ResourceKeySyntaxV1::GltfUri).unwrap();
        let mut external_builder =
            DependencyClosureBuilderV1::new(artifact, SourceSetCoverageV1::complete(), 1);
        assert!(external_builder.begin_reference(12, 1));
        assert_eq!(
            external_builder.prepare_external_key(&key).unwrap(),
            Some(true)
        );
        external_builder.record_external_open_attempt(&key).unwrap();
        assert!(
            external_builder
                .push_captured_external(
                    0,
                    SourceResourceKindV1::Buffer,
                    0,
                    key,
                    InputIdentity::from_bytes(b"external"),
                )
                .unwrap()
        );
        let external = external_builder.finish().unwrap();
        assert!(!is_self_contained_closure(&external));
    }

    #[test]
    fn circular_contact_and_signed_yaw_boundaries_are_inclusive() {
        assert!(circular_phase_distance(0.995, 0.005) <= 0.01000000000000002);
        assert_eq!(signed_angle_distance(179.5, -179.5).abs(), 1.0);
        assert!(prove_root_values([0.0, 0.0, -1.0], [0.01, 0.0, 0.0]).is_ok());
        assert_eq!(
            prove_root_values([0.015, 0.0, 0.0], [0.007, 0.0, 0.0])
                .unwrap_err()
                .kind(),
            FootCycleProofKind::RootTrajectory
        );
        assert_eq!(
            prove_root_values(
                [0.0, 0.0, 0.0],
                [f64::from_bits(0.01f64.to_bits() + 1), 0.0, 0.0]
            )
            .unwrap_err()
            .kind(),
            FootCycleProofKind::RootTrajectory
        );
    }

    #[test]
    fn contact_proof_names_topology_and_boundary_mutations() {
        let expected = contact_fragment(&[
            (ContactRoleV1::LeftFoot, 0.125, 0.25),
            (ContactRoleV1::RightFoot, 0.625, 0.75),
        ]);
        let within = contact_fragment(&[
            (ContactRoleV1::LeftFoot, 0.1328125, 0.2578125),
            (ContactRoleV1::RightFoot, 0.6328125, 0.7578125),
        ]);
        assert_eq!(
            prove_contact_boundaries(&expected, &within, 0.0078125).unwrap(),
            0.0078125
        );
        let phase_warped = contact_fragment(&[
            (ContactRoleV1::LeftFoot, 0.25, 0.375),
            (ContactRoleV1::RightFoot, 0.75, 0.875),
        ]);
        assert_eq!(
            prove_contact_boundaries(&expected, &phase_warped, 0.5).unwrap(),
            0.125
        );
        let wrong_side = contact_fragment(&[
            (ContactRoleV1::RightFoot, 0.125, 0.25),
            (ContactRoleV1::LeftFoot, 0.625, 0.75),
        ]);
        assert_eq!(
            prove_contact_boundaries(&expected, &wrong_side, 0.5)
                .unwrap_err()
                .kind(),
            FootCycleProofKind::ContactTopology
        );
        let outside = contact_fragment(&[
            (ContactRoleV1::LeftFoot, 0.140625, 0.265625),
            (ContactRoleV1::RightFoot, 0.640625, 0.765625),
        ]);
        assert_eq!(
            prove_contact_boundaries(&expected, &outside, 0.0078125)
                .unwrap_err()
                .kind(),
            FootCycleProofKind::ContactBoundary
        );
    }

    #[test]
    fn contact_proof_accepts_overlap_and_seam_but_rejects_same_side_overlap() {
        let overlap = contact_fragment(&[
            (ContactRoleV1::LeftFoot, 0.1, 0.45),
            (ContactRoleV1::RightFoot, 0.3, 0.7),
        ]);
        let shifted_overlap = contact_fragment(&[
            (ContactRoleV1::LeftFoot, 0.11, 0.46),
            (ContactRoleV1::RightFoot, 0.31, 0.71),
        ]);
        assert!(prove_contact_boundaries(&overlap, &shifted_overlap, 0.011).is_ok());

        let seam = contact_fragment(&[
            (ContactRoleV1::LeftFoot, 0.0, 0.3),
            (ContactRoleV1::RightFoot, 0.2, 0.8),
            (ContactRoleV1::LeftFoot, 0.7, 1.0),
        ]);
        let shifted_seam = contact_fragment(&[
            (ContactRoleV1::LeftFoot, 0.0, 0.31),
            (ContactRoleV1::RightFoot, 0.21, 0.81),
            (ContactRoleV1::LeftFoot, 0.71, 1.0),
        ]);
        assert!(prove_contact_boundaries(&seam, &shifted_seam, 0.011).is_ok());

        let both_seams = contact_fragment(&[
            (ContactRoleV1::LeftFoot, 0.0, 0.2),
            (ContactRoleV1::RightFoot, 0.0, 0.3),
            (ContactRoleV1::LeftFoot, 0.7, 1.0),
            (ContactRoleV1::RightFoot, 0.8, 1.0),
        ]);
        let shifted_both_seams = contact_fragment(&[
            (ContactRoleV1::LeftFoot, 0.0, 0.21),
            (ContactRoleV1::RightFoot, 0.0, 0.31),
            (ContactRoleV1::LeftFoot, 0.71, 1.0),
            (ContactRoleV1::RightFoot, 0.81, 1.0),
        ]);
        assert!(prove_contact_boundaries(&both_seams, &shifted_both_seams, 0.011).is_ok());

        let simultaneous = contact_fragment(&[
            (ContactRoleV1::LeftFoot, 0.1, 0.3),
            (ContactRoleV1::RightFoot, 0.3, 0.6),
        ]);
        let shifted_simultaneous = contact_fragment(&[
            (ContactRoleV1::LeftFoot, 0.11, 0.31),
            (ContactRoleV1::RightFoot, 0.31, 0.61),
        ]);
        assert!(prove_contact_boundaries(&simultaneous, &shifted_simultaneous, 0.011).is_ok());

        let malformed = contact_fragment(&[
            (ContactRoleV1::LeftFoot, 0.1, 0.3),
            (ContactRoleV1::LeftFoot, 0.2, 0.4),
            (ContactRoleV1::RightFoot, 0.5, 0.6),
            (ContactRoleV1::RightFoot, 0.7, 0.8),
        ]);
        assert_eq!(
            prove_contact_boundaries(&malformed, &malformed, 0.5)
                .unwrap_err()
                .kind(),
            FootCycleProofKind::ContactTopology
        );

        let touching = contact_fragment(&[
            (ContactRoleV1::LeftFoot, 0.1, 0.3),
            (ContactRoleV1::RightFoot, 0.3, 0.6),
        ]);
        let separated = contact_fragment(&[
            (ContactRoleV1::LeftFoot, 0.1, 0.29),
            (ContactRoleV1::RightFoot, 0.31, 0.6),
        ]);
        assert_eq!(
            prove_contact_boundaries(&touching, &separated, 0.02)
                .unwrap_err()
                .kind(),
            FootCycleProofKind::ContactTopology
        );
    }

    #[test]
    fn clip_map_proof_rejects_one_output_time_mutation() {
        let prepared = crate::foot_cycle_source_prep::tests::prepared_fixture_for_proof_tests();
        let member = &prepared.members()[0];
        let source =
            &prepared.sources()[member.source_index()].document().clips[member.clip_index()];
        let operation = prepared.plan().members()[0].operation();
        prove_clip_map(source, member.candidate_clip(), operation).unwrap();

        let mut mutated = member.candidate_clip().clone();
        mutated.tracks[0].times[1] = f32::from_bits(mutated.tracks[0].times[1].to_bits() + 1);
        assert_eq!(
            prove_clip_map(source, &mutated, operation)
                .unwrap_err()
                .kind(),
            FootCycleProofKind::ClipMap
        );

        let mut nonmonotone = operation.clone();
        let ContactTransformOperationV1::TimeWarp { control_points, .. } = &mut nonmonotone else {
            unreachable!()
        };
        control_points.swap(0, 1);
        assert_eq!(
            prove_clip_map(source, member.candidate_clip(), &nonmonotone)
                .unwrap_err()
                .kind(),
            FootCycleProofKind::ClipMap
        );
    }

    /// A control point that reconstructs one binary32 place from its authored
    /// key is that key.
    ///
    /// The proof expects one emitted key there, carrying the control point's
    /// own output, and refuses both the extra knot the builder emitted before
    /// this contract and a key that kept the mapped authored output.
    #[test]
    fn clip_map_proof_expects_one_key_per_coincident_control_point() {
        let frames = 18_usize;
        let duration = f64::from(17.0_f32 / 30.0);
        let narrowed_duration = duration as f32;
        let times = (0..frames).map(|key| key as f32 / 30.0).collect::<Vec<_>>();
        let values = (0..frames)
            .map(|key| animsmith_core::glam::Vec3::splat(key as f32))
            .collect::<Vec<_>>();
        let track = |times: Vec<f32>, values: Vec<animsmith_core::glam::Vec3>| Track {
            bone: 0,
            property: animsmith_core::Property::Translation,
            interpolation: Interpolation::Linear,
            times,
            values: TrackValues::Vec3s(values),
        };
        let clip = |tracks: Vec<Track>| animsmith_core::Clip {
            name: "walk".to_owned(),
            duration_s: duration,
            tracks,
        };
        let source = clip(vec![track(times.clone(), values.clone())]);

        let phase = |frame: usize| frame as f64 / (frames - 1) as f64;
        let control_points = vec![
            ContactTimeWarpControlPointV1::new(0.0, 0.0),
            ContactTimeWarpControlPointV1::new(phase(4), 0.05),
            ContactTimeWarpControlPointV1::new(phase(5), 0.9),
            ContactTimeWarpControlPointV1::new(1.0, 1.0),
        ];
        let operation = ContactTransformOperationV1::time_warp(duration, control_points.clone());
        let narrow = |normalized: f64| (normalized * f64::from(narrowed_duration)) as f32;

        // Both coincidences the contract admits are present: reconstructing
        // the first control point lands one binary32 place below its authored
        // key, and the second reproduces its authored key exactly.
        assert_eq!(narrow(phase(4)).next_up(), times[4]);
        assert_eq!(narrow(phase(5)), times[5]);
        // The segment leaving the first control point is steep, so the map
        // recomputed through the authored key is far from the control point's
        // own output rather than a rounding step away from it.
        let coalesced = [narrow(0.05), narrow(0.9)];
        let mapped = map_source_time(times[4], narrowed_duration, &control_points).unwrap();
        assert_ne!(mapped, coalesced[0]);

        let output_times = times
            .iter()
            .enumerate()
            .map(|(key, &time)| match key {
                4 => coalesced[0],
                5 => coalesced[1],
                _ => map_source_time(time, narrowed_duration, &control_points).unwrap(),
            })
            .collect::<Vec<_>>();
        let candidate = clip(vec![track(output_times.clone(), values.clone())]);
        prove_clip_map(&source, &candidate, &operation).unwrap();

        let mut extra_times = output_times.clone();
        let mut extra_values = values.clone();
        extra_times[4] = mapped;
        extra_times.insert(4, coalesced[0]);
        extra_values.insert(
            4,
            match sample_track(&source.tracks[0], narrow(phase(4))) {
                TrackSample::Vec3(value) => value,
                TrackSample::Quat(_) => unreachable!("translation track"),
            },
        );
        assert_eq!(
            prove_clip_map(
                &source,
                &clip(vec![track(extra_times, extra_values)]),
                &operation
            )
            .unwrap_err()
            .kind(),
            FootCycleProofKind::ClipMap,
            "an extra knot beside the authored key is a second spelling of one instant"
        );

        let mut mapped_output = output_times;
        mapped_output[4] = mapped;
        assert!(mapped_output.windows(2).all(|pair| pair[0] < pair[1]));
        assert_eq!(
            prove_clip_map(
                &source,
                &clip(vec![track(mapped_output, values)]),
                &operation
            )
            .unwrap_err()
            .kind(),
            FootCycleProofKind::ClipMap,
            "the coalesced key carries the control point output, not the mapped one"
        );
    }

    #[test]
    fn clip_map_proof_accepts_step_tracks_with_authored_breakpoints_only() {
        let prepared = crate::foot_cycle_source_prep::tests::prepared_fixture_for_proof_tests();
        let member = &prepared.members()[1];
        let mut source =
            prepared.sources()[member.source_index()].document().clips[member.clip_index()].clone();
        source.tracks[0].interpolation = Interpolation::Step;
        let plan = &prepared.plan().members()[1];
        let candidate = animsmith_core::time_warp_clip_v1(&source, plan).unwrap();
        assert_eq!(
            candidate.tracks[0].times.len(),
            source.tracks[0].times.len()
        );
        prove_clip_map(&source, &candidate, plan.operation()).unwrap();
    }

    #[test]
    fn clip_map_proof_rejects_mutated_linear_and_step_output_values() {
        let prepared = crate::foot_cycle_source_prep::tests::prepared_fixture_for_proof_tests();
        let member = &prepared.members()[1];
        let plan = &prepared.plan().members()[1];
        for interpolation in [Interpolation::Linear, Interpolation::Step] {
            let mut source = prepared.sources()[member.source_index()].document().clips
                [member.clip_index()]
            .clone();
            source.tracks[0].interpolation = interpolation;
            let candidate = animsmith_core::time_warp_clip_v1(&source, plan).unwrap();
            prove_clip_map(&source, &candidate, plan.operation()).unwrap();

            let mut mutated = candidate;
            let TrackValues::Vec3s(values) = &mut mutated.tracks[0].values else {
                panic!("fixture translation track");
            };
            values[1].x = f32::from_bits(values[1].x.to_bits() + 1);
            assert_eq!(
                prove_clip_map(&source, &mutated, plan.operation())
                    .unwrap_err()
                    .kind(),
                FootCycleProofKind::ClipMap
            );
        }
    }

    #[test]
    fn clip_map_proof_accepts_nonidentity_constant_multi_key_cubic_motion() {
        let prepared = crate::foot_cycle_source_prep::tests::prepared_fixture_for_proof_tests();
        let member = &prepared.members()[1];
        let plan = &prepared.plan().members()[1];
        let mut source =
            prepared.sources()[member.source_index()].document().clips[member.clip_index()].clone();
        let track = &mut source.tracks[0];
        assert!(track.times.len() > 1);
        let TrackValues::Vec3s(authored) = &track.values else {
            panic!("fixture translation track");
        };
        let value = authored[0];
        let mut zero = value;
        zero.x = 0.0;
        zero.y = 0.0;
        zero.z = 0.0;
        track.interpolation = Interpolation::CubicSpline;
        track.values = TrackValues::Vec3s(
            track
                .times
                .iter()
                .flat_map(|_| [zero, value, zero])
                .collect(),
        );
        let ContactTransformOperationV1::TimeWarp { control_points, .. } = plan.operation() else {
            unreachable!()
        };
        assert!(
            control_points
                .iter()
                .any(|point| point.input_time() != point.output_time())
        );

        let candidate = animsmith_core::time_warp_clip_v1(&source, plan).unwrap();
        assert!(tracks_equal_bits(&source.tracks[0], &candidate.tracks[0]));
        prove_clip_map(&source, &candidate, plan.operation()).unwrap();
    }

    #[test]
    fn clip_map_proof_independently_refuses_bit_identical_nonconstant_cubic_motion() {
        let prepared = crate::foot_cycle_source_prep::tests::prepared_fixture_for_proof_tests();
        let member = &prepared.members()[1];
        let mut source =
            prepared.sources()[member.source_index()].document().clips[member.clip_index()].clone();
        let track = &mut source.tracks[0];
        let TrackValues::Vec3s(authored) = &track.values else {
            panic!("fixture translation track");
        };
        let mut zero = authored[0];
        zero.x = 0.0;
        zero.y = 0.0;
        zero.z = 0.0;
        let mut cubic = authored
            .iter()
            .flat_map(|value| [zero, *value, zero])
            .collect::<Vec<_>>();
        cubic[4].x += 1.0;
        track.interpolation = Interpolation::CubicSpline;
        track.values = TrackValues::Vec3s(cubic);

        assert_eq!(
            prove_clip_map(
                &source,
                &source.clone(),
                prepared.plan().members()[1].operation()
            )
            .unwrap_err()
            .kind(),
            FootCycleProofKind::ClipMap
        );
    }

    #[test]
    fn f32_derived_loop_caps_accept_the_exact_user_boundary() {
        let measured = f64::from(0.1f32);
        assert!(!exceeds_f32_cap(measured, 0.1));
        assert!(exceeds_f32_cap(
            f64::from(f32::from_bits(0.1f32.to_bits() + 1)),
            0.1
        ));
    }

    #[test]
    fn gait_spread_accepts_the_exact_circular_boundary_and_rejects_one_bit_over() {
        assert_eq!(prove_gait_spread(&[0.0, 0.25], 0.125).unwrap(), 0.125);
        assert_eq!(
            prove_gait_spread(&[0.0, f64::from_bits(0.25f64.to_bits() + 1)], 0.125)
                .unwrap_err()
                .kind(),
            FootCycleProofKind::GaitSpread
        );
        assert!(prove_gait_spread(&[0.99, 0.01], 0.02000000000000002).is_ok());
    }

    #[test]
    fn every_preflight_finishes_before_the_first_candidate_write() {
        let prepared = crate::foot_cycle_source_prep::tests::prepared_fixture_for_proof_tests();
        let mut runtime = ObservedRuntime {
            fail_preflight_at: Some(1),
            ..ObservedRuntime::default()
        };
        let error = serialize_and_prove_foot_cycle_v1_with_runtime(&prepared, &mut runtime)
            .err()
            .expect("second preflight must fail");
        assert_eq!(error.kind(), FootCycleProofKind::ArtifactPreflight);
        assert_eq!(runtime.preflights, 2);
        assert_eq!(runtime.writes, 0);
        assert_eq!(runtime.readbacks, 0);
    }

    #[test]
    fn second_write_failure_returns_no_partial_batch_or_readback() {
        let prepared = crate::foot_cycle_source_prep::tests::prepared_fixture_for_proof_tests();
        let mut runtime = ObservedRuntime {
            fail_write_at: Some(1),
            ..ObservedRuntime::default()
        };
        let error = serialize_and_prove_foot_cycle_v1_with_runtime(&prepared, &mut runtime)
            .err()
            .expect("second write must fail the whole batch");
        assert_eq!(error.kind(), FootCycleProofKind::ArtifactWrite);
        assert_eq!(runtime.preflights, 2);
        assert_eq!(runtime.writes, 2);
        assert_eq!(runtime.readbacks, 0);
    }

    #[test]
    fn second_member_proof_failure_returns_no_partial_batch() {
        let prepared = crate::foot_cycle_source_prep::tests::proof_ready_fixture();
        let mut runtime = ObservedRuntime {
            grid_mutation: Some((1, GridMutation::ContactTopology)),
            ..ObservedRuntime::default()
        };
        let error = serialize_and_prove_foot_cycle_v1_with_runtime(&prepared, &mut runtime)
            .err()
            .expect("second proof mutation must fail the whole batch");
        assert_eq!(error.kind(), FootCycleProofKind::ContactTopology);
        assert_eq!(runtime.grids_built, 2);
    }

    #[test]
    fn gait_domain_independently_rejects_a_below_floor_output_grid() {
        let prepared = crate::foot_cycle_source_prep::tests::proof_ready_fixture();
        let member = &prepared.members()[0];
        let source = &prepared.sources()[member.source_index()];
        let mut document = source.document().clone();
        document.clips[member.clip_index()] = member.candidate_clip().clone();
        let mut runtime = ObservedRuntime {
            grid_mutation: Some((0, GridMutation::LowGaitAmplitude)),
            ..ObservedRuntime::default()
        };
        let grid = runtime.build_grid(&document, member.clip_index()).unwrap();
        let roles = resolve_configured_roles(&document.skeleton, &source.config().rig);
        assert_eq!(
            prove_gait(
                &grid,
                &roles,
                source.config().loop_seam_min_stride_step_m(),
                prepared.plan().proof().min_lr_amplitude_m(),
            )
            .unwrap_err()
            .kind(),
            FootCycleProofKind::GaitAmplitude
        );
    }

    #[test]
    fn duration_domain_rejects_a_mutated_output_duration() {
        let prepared = crate::foot_cycle_source_prep::tests::proof_ready_fixture();
        let member = &prepared.members()[0];
        let source =
            &prepared.sources()[member.source_index()].document().clips[member.clip_index()];
        let mut output = member.candidate_clip().clone();
        output.duration_s = 2.0;
        let contact = contact_fragment(&[(ContactRoleV1::LeftFoot, 0.1, 0.2)]);
        assert_eq!(
            prove_duration(source.duration_s, &output, &contact, &contact)
                .unwrap_err()
                .kind(),
            FootCycleProofKind::Duration
        );
    }

    #[test]
    fn transaction_rejects_independently_mutated_root_and_loop_metrics() {
        for (mutation, expected) in [
            (
                GridMutation::RootTrajectory,
                FootCycleProofKind::RootTrajectory,
            ),
            (
                GridMutation::LoopContinuity,
                FootCycleProofKind::LoopContinuity,
            ),
        ] {
            let prepared = crate::foot_cycle_source_prep::tests::proof_ready_fixture();
            let mut runtime = ObservedRuntime {
                grid_mutation: Some((1, mutation)),
                ..ObservedRuntime::default()
            };
            let error = serialize_and_prove_foot_cycle_v1_with_runtime(&prepared, &mut runtime)
                .err()
                .expect("mutated output proof grid must fail the whole transaction");
            assert_eq!(error.kind(), expected);
            assert_eq!(runtime.grids_built, 2);
        }
    }

    #[test]
    fn transaction_rejects_mutated_duration_and_map_without_partial_results() {
        for (prepared, expected) in [
            (
                crate::foot_cycle_source_prep::tests::proof_ready_fixture_with_candidate_duration_mutation(),
                FootCycleProofKind::ArtifactPreflight,
            ),
            (
                crate::foot_cycle_source_prep::tests::proof_ready_fixture_with_candidate_map_mutation(),
                FootCycleProofKind::ClipMap,
            ),
        ] {
            let error = serialize_and_prove_foot_cycle_v1(&prepared)
                .err()
                .expect("mutated candidate must fail the whole transaction");
            assert_eq!(error.kind(), expected);
        }
    }

    #[test]
    fn combined_source_and_output_metric_budget_is_inclusive() {
        const OUTPUT_WORK: usize = 2 * 17 * 4;
        let exact =
            crate::foot_cycle_source_prep::tests::proof_ready_fixture_with_source_metric_work(
                MAX_METRIC_GRID_WORK - OUTPUT_WORK,
                MAX_METRIC_GRID_WORK - OUTPUT_WORK,
            );
        let proved = serialize_and_prove_foot_cycle_v1(&exact).unwrap();
        assert_eq!(proved.metric_pose_cells(), MAX_METRIC_GRID_WORK);
        assert_eq!(proved.metric_sample_evaluations(), MAX_METRIC_GRID_WORK);
        assert_eq!(proved.output_metric_pose_cells(), OUTPUT_WORK);
        assert_eq!(proved.output_metric_sample_evaluations(), OUTPUT_WORK);

        let over =
            crate::foot_cycle_source_prep::tests::proof_ready_fixture_with_source_metric_work(
                MAX_METRIC_GRID_WORK - OUTPUT_WORK + 1,
                MAX_METRIC_GRID_WORK - OUTPUT_WORK,
            );
        let mut runtime = ObservedRuntime::default();
        let error = serialize_and_prove_foot_cycle_v1_with_runtime(&over, &mut runtime)
            .err()
            .expect("one combined pose cell over the cap must fail");
        assert_eq!(error.kind(), FootCycleProofKind::MetricWork);
        assert_eq!(runtime.grids_built, 0);
        assert_eq!(runtime.writes, 0);
        assert_eq!(runtime.readbacks, 0);
    }

    #[test]
    fn valid_json_generator_mutation_is_caught_by_exact_artifact_identity() {
        let prepared = crate::foot_cycle_source_prep::tests::prepared_fixture_for_proof_tests();
        let mut runtime = ObservedRuntime {
            mutate_readback_at: Some(0),
            ..ObservedRuntime::default()
        };
        let error = serialize_and_prove_foot_cycle_v1_with_runtime(&prepared, &mut runtime)
            .err()
            .expect("readback of different exact bytes must fail");
        assert_eq!(error.kind(), FootCycleProofKind::ArtifactIdentity);
        assert_eq!(runtime.preflights, 2);
        assert_eq!(runtime.writes, 2);
        assert_eq!(runtime.readbacks, 1);
    }

    #[test]
    fn production_transaction_proves_the_complete_in_memory_batch() {
        let prepared =
            crate::foot_cycle_source_prep::tests::proof_ready_fixture_with_nonzero_selected_clips();
        let proved = serialize_and_prove_foot_cycle_v1(&prepared).unwrap();
        assert_eq!(proved.members().len(), 2);
        assert_eq!(
            proved.retained_candidate_bytes(),
            proved
                .members()
                .iter()
                .map(|member| member.artifact_bytes().len())
                .sum::<usize>()
        );
        assert_eq!(proved.metric_pose_cells(), 4 * 17 * 4);
        assert_eq!(proved.metric_sample_evaluations(), 4 * 17 * 4);
        assert_eq!(proved.source_metric_pose_cells(), 2 * 17 * 4);
        assert_eq!(proved.source_metric_sample_evaluations(), 2 * 17 * 4);
        assert_eq!(proved.output_metric_pose_cells(), 2 * 17 * 4);
        assert_eq!(proved.output_metric_sample_evaluations(), 2 * 17 * 4);
        assert!(proved.gait_phase_spread() <= 0.08);
        for (member_index, member) in proved.members().iter().enumerate() {
            let prepared_member = &prepared.members()[member_index];
            assert_eq!(prepared_member.clip_index(), 1);
            let source = &prepared.sources()[prepared_member.source_index()];
            let reread = animsmith_gltf::load_source_bytes(
                Path::new("proved-artifact.glb"),
                member.artifact_bytes(),
            )
            .unwrap();
            assert!(clips_equal_bits(
                &reread.document().clips[0],
                &source.document().clips[0]
            ));
            assert!(clips_equal_bits(
                &reread.document().clips[prepared_member.clip_index()],
                prepared_member.candidate_clip()
            ));
            assert_eq!(
                member.artifact(),
                &InputIdentity::from_bytes(member.artifact_bytes())
            );
            assert!(member.dependency_closure().coverage().is_complete());
            assert!(
                member
                    .dependency_closure()
                    .references()
                    .iter()
                    .all(|reference| reference.target() == &DependencyReferenceTargetV1::Primary)
            );
            assert!(member.dependency_closure().external_resources().is_empty());
            assert_eq!(
                member.contact_transform().output().unwrap().artifact(),
                member.artifact()
            );
            assert_eq!(
                member.independently_detected_contact().artifact(),
                member.artifact()
            );
            assert_eq!(member.facts().duration_s(), 1.0);
            assert!(member.facts().gait_phase().is_finite());
            assert!(member.facts().lr_amplitude_m() >= 0.05);
            assert_eq!(member.facts().max_contact_boundary_phase_error(), 0.0);
            assert!(member.facts().root_endpoint_displacement_x_m().is_finite());
            assert!(member.facts().root_endpoint_displacement_z_m().is_finite());
            assert!(member.facts().root_accumulated_yaw_deg().is_finite());
            assert!(member.facts().max_loop_position_delta_m().is_finite());
            assert!(member.facts().max_loop_rotation_delta_deg().is_finite());
            assert!(member.facts().max_loop_velocity_delta_mps().is_finite());
            assert!(
                member
                    .facts()
                    .max_loop_angular_velocity_delta_degps()
                    .is_finite()
            );
        }
    }
}
