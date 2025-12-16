use anyhow::Result;
use rapier2d::prelude::*;
use std::collections::HashMap;

use crate::math::Vec2;
use crate::world::EntityId;

/// Callback function type for collision events.
pub type CollisionCallback = Box<dyn Fn(EntityId, EntityId, bool) + Send + Sync>;

/// Physics world that manages rigid bodies, colliders, and the simulation.
///
/// This wraps Rapier2D's physics world and provides a Forge2D-friendly API
/// that integrates with the `World`/`EntityId` system.
pub struct PhysicsWorld {
    /// The underlying Rapier physics pipeline
    pipeline: PhysicsPipeline,
    /// The physics world containing all bodies and colliders
    physics_world: RigidBodySet,
    collider_set: ColliderSet,
    /// Query pipeline for efficient spatial queries
    query_pipeline: QueryPipeline,
    /// Island manager for sleeping bodies
    island_manager: IslandManager,
    /// Broad phase for collision detection
    broad_phase: BroadPhase,
    /// Narrow phase for collision detection
    narrow_phase: NarrowPhase,
    /// Impulse joint set
    impulse_joint_set: ImpulseJointSet,
    /// Multibody joint set
    multibody_joint_set: MultibodyJointSet,
    /// CCD solver for continuous collision detection
    ccd_solver: CCDSolver,
    /// Mapping from EntityId to Rapier RigidBodyHandle
    entity_to_body: HashMap<EntityId, RigidBodyHandle>,
    /// Mapping from Rapier RigidBodyHandle to EntityId
    body_to_entity: HashMap<RigidBodyHandle, EntityId>,
    /// Gravity vector (default: (0, 9.81) for downward gravity)
    pub gravity: Vec2,
    /// Integration parameters
    integration_parameters: IntegrationParameters,
    /// Collision callbacks
    collision_callbacks: Vec<CollisionCallback>,
    /// Track active collisions for start/stop events
    active_collisions: HashMap<(EntityId, EntityId), bool>,
}

impl PhysicsWorld {
    /// Create a new physics world with default settings.
    pub fn new() -> Self {
        Self {
            pipeline: PhysicsPipeline::new(),
            physics_world: RigidBodySet::new(),
            collider_set: ColliderSet::new(),
            query_pipeline: QueryPipeline::new(),
            island_manager: IslandManager::new(),
            broad_phase: BroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            impulse_joint_set: ImpulseJointSet::new(),
            multibody_joint_set: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(),
            entity_to_body: HashMap::new(),
            body_to_entity: HashMap::new(),
            gravity: Vec2::new(0.0, 9.81),
            integration_parameters: IntegrationParameters::default(),
            collision_callbacks: Vec::new(),
            active_collisions: HashMap::new(),
        }
    }

    /// Create a new physics world with custom gravity.
    pub fn with_gravity(gravity: Vec2) -> Self {
        Self {
            gravity,
            ..Self::new()
        }
    }

    /// Set the gravity vector.
    pub fn set_gravity(&mut self, gravity: Vec2) {
        self.gravity = gravity;
    }

    /// Step the physics simulation forward by the given timestep.
    ///
    /// This should be called in your fixed timestep update loop.
    /// The timestep should be consistent (e.g., 1/60 seconds for 60 FPS).
    pub fn step(&mut self, timestep: f32) {
        // Update integration parameters with the timestep
        self.integration_parameters.dt = timestep;

        // Create a gravity vector for Rapier
        let gravity = vector![self.gravity.x, self.gravity.y];

        // Step the physics simulation
        let hooks = &();
        let event_handler = &mut ();
        
        self.pipeline.step(
            &gravity,
            &self.integration_parameters,
            &mut self.island_manager,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.physics_world,
            &mut self.collider_set,
            &mut self.impulse_joint_set,
            &mut self.multibody_joint_set,
            &mut self.ccd_solver,
            hooks,
            event_handler,
        );
        
        // Check for collision start/stop events by querying contacts
        self.update_collision_events();
        
        // Update query pipeline after physics step
        self.query_pipeline.update(&self.island_manager, &self.physics_world, &self.collider_set);
    }

    /// Internal method to detect collision start/stop events.
    fn update_collision_events(&mut self) {
        if self.collision_callbacks.is_empty() {
            return;
        }

        // Get all contact pairs from the narrow phase
        let mut current_collisions = HashMap::new();
        
        for (handle1, collider1) in self.collider_set.iter() {
            if let Some(body1) = collider1.parent() {
                if let Some(entity1) = self.get_entity(body1) {
                    for (handle2, collider2) in self.collider_set.iter() {
                        if handle1 == handle2 {
                            continue;
                        }
                        if let Some(body2) = collider2.parent() {
                            if let Some(entity2) = self.get_entity(body2) {
                                // Check if these colliders are in contact
                                if self.narrow_phase.contact_pair(handle1, handle2).is_some() {
                                    let key = if entity1.to_u32() < entity2.to_u32() {
                                        (entity1, entity2)
                                    } else {
                                        (entity2, entity1)
                                    };
                                    current_collisions.insert(key, true);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Detect collision start events
        for (key, _) in &current_collisions {
            if !self.active_collisions.contains_key(key) {
                // Collision started
                let callbacks = &self.collision_callbacks;
                for callback in callbacks {
                    callback(key.0, key.1, true);
                }
            }
        }

        // Detect collision stop events
        for (key, _) in &self.active_collisions {
            if !current_collisions.contains_key(key) {
                // Collision stopped
                let callbacks = &self.collision_callbacks;
                for callback in callbacks {
                    callback(key.0, key.1, false);
                }
            }
        }

        self.active_collisions = current_collisions;
    }

    /// Register a callback function that will be called when collisions start or stop.
    ///
    /// The callback receives: `(entity1, entity2, started)` where `started` is `true` for
    /// collision start and `false` for collision end.
    pub fn on_collision<F>(&mut self, callback: F)
    where
        F: Fn(EntityId, EntityId, bool) + Send + Sync + 'static,
    {
        self.collision_callbacks.push(Box::new(callback));
    }

    /// Create a rigid body for an entity.
    ///
    /// `body_type` can be:
    /// - `RigidBodyType::Dynamic` - affected by forces and collisions
    /// - `RigidBodyType::Kinematic` - moved manually, not affected by forces
    /// - `RigidBodyType::Fixed` - static, immovable
    pub fn create_body(
        &mut self,
        entity: EntityId,
        body_type: RigidBodyType,
        position: Vec2,
        rotation: f32,
    ) -> Result<RigidBodyHandle> {
        // Remove existing body if any
        if let Some(old_handle) = self.entity_to_body.remove(&entity) {
            self.physics_world.remove(
                old_handle,
                &mut self.island_manager,
                &mut self.collider_set,
                &mut self.impulse_joint_set,
                &mut self.multibody_joint_set,
                true,
            );
            self.body_to_entity.remove(&old_handle);
        }

        // Create rigid body builder
        let mut builder = RigidBodyBuilder::new(body_type);
        builder = builder.translation(vector![position.x, position.y]);
        builder = builder.rotation(rotation);

        let body = builder.build();
        let handle = self.physics_world.insert(body);
        self.entity_to_body.insert(entity, handle);
        self.body_to_entity.insert(handle, entity);

        Ok(handle)
    }


    /// Add a collider to a rigid body.
    ///
    /// `shape` can be created using helper methods like `create_box_collider`,
    /// `create_circle_collider`, etc.
    pub fn add_collider(
        &mut self,
        body_handle: RigidBodyHandle,
        shape: SharedShape,
        offset: Vec2,
        density: f32,
    ) -> ColliderHandle {
        self.add_collider_with_material(body_handle, shape, offset, density, 0.5, 0.0)
    }

    /// Add a collider with material properties (friction and restitution/bounciness).
    ///
    /// - `friction`: Coefficient of friction (0.0 = no friction, 1.0 = high friction)
    /// - `restitution`: Bounciness (0.0 = no bounce, 1.0 = fully elastic)
    pub fn add_collider_with_material(
        &mut self,
        body_handle: RigidBodyHandle,
        shape: SharedShape,
        offset: Vec2,
        density: f32,
        friction: f32,
        restitution: f32,
    ) -> ColliderHandle {
        let mut collider_builder = ColliderBuilder::new(shape);
        collider_builder = collider_builder.translation(vector![offset.x, offset.y]);
        collider_builder = collider_builder.density(density);
        collider_builder = collider_builder.friction(friction);
        collider_builder = collider_builder.restitution(restitution);

        let collider = collider_builder.build();
        self.collider_set.insert_with_parent(collider, body_handle, &mut self.physics_world)
    }

    /// Add a sensor (trigger volume) that detects collisions but doesn't physically collide.
    ///
    /// Sensors are useful for trigger zones, pickup detection, etc.
    pub fn add_sensor(
        &mut self,
        body_handle: RigidBodyHandle,
        shape: SharedShape,
        offset: Vec2,
    ) -> ColliderHandle {
        let mut collider_builder = ColliderBuilder::new(shape);
        collider_builder = collider_builder.translation(vector![offset.x, offset.y]);
        collider_builder = collider_builder.sensor(true); // Make it a sensor

        let collider = collider_builder.build();
        self.collider_set.insert_with_parent(collider, body_handle, &mut self.physics_world)
    }

    /// Get the position of a rigid body.
    pub fn get_body_position(&self, body_handle: RigidBodyHandle) -> Option<Vec2> {
        self.physics_world
            .get(body_handle)
            .map(|body| {
                let trans = body.translation();
                Vec2::new(trans.x, trans.y)
            })
    }

    /// Get the rotation of a rigid body (in radians).
    pub fn get_body_rotation(&self, body_handle: RigidBodyHandle) -> Option<f32> {
        self.physics_world
            .get(body_handle)
            .map(|body| body.rotation().angle())
    }

    /// Set the position of a rigid body.
    pub fn set_body_position(&mut self, body_handle: RigidBodyHandle, position: Vec2) {
        if let Some(body) = self.physics_world.get_mut(body_handle) {
            body.set_translation(vector![position.x, position.y], true);
        }
    }

    /// Set the rotation of a rigid body (in radians).
    pub fn set_body_rotation(&mut self, body_handle: RigidBodyHandle, rotation: f32) {
        if let Some(body) = self.physics_world.get_mut(body_handle) {
            body.set_rotation(rotation, true);
        }
    }

    /// Apply a linear velocity to a rigid body.
    pub fn set_linear_velocity(&mut self, body_handle: RigidBodyHandle, velocity: Vec2) {
        if let Some(body) = self.physics_world.get_mut(body_handle) {
            body.set_linvel(vector![velocity.x, velocity.y], true);
        }
    }

    /// Apply an impulse (instantaneous force) to a rigid body.
    pub fn apply_impulse(&mut self, body_handle: RigidBodyHandle, impulse: Vec2) {
        if let Some(body) = self.physics_world.get_mut(body_handle) {
            let current_linvel = body.linvel();
            body.set_linvel(
                vector![current_linvel.x + impulse.x, current_linvel.y + impulse.y],
                true,
            );
        }
    }

    /// Apply a continuous force to a rigid body (call each frame for sustained force).
    pub fn apply_force(&mut self, body_handle: RigidBodyHandle, force: Vec2) {
        if let Some(body) = self.physics_world.get_mut(body_handle) {
            body.add_force(vector![force.x, force.y], true);
        }
    }

    /// Apply a force at a specific point (useful for torque effects).
    pub fn apply_force_at_point(
        &mut self,
        body_handle: RigidBodyHandle,
        force: Vec2,
        point: Vec2,
    ) {
        if let Some(body) = self.physics_world.get_mut(body_handle) {
            body.add_force_at_point(
                vector![force.x, force.y],
                point![point.x, point.y],
                true,
            );
        }
    }

    /// Get the linear velocity of a rigid body.
    pub fn get_linear_velocity(&self, body_handle: RigidBodyHandle) -> Option<Vec2> {
        self.physics_world
            .get(body_handle)
            .map(|body| {
                let vel = body.linvel();
                Vec2::new(vel.x, vel.y)
            })
    }

    /// Set the angular velocity of a rigid body (rotation speed in radians per second).
    pub fn set_angular_velocity(&mut self, body_handle: RigidBodyHandle, angular_velocity: f32) {
        if let Some(body) = self.physics_world.get_mut(body_handle) {
            body.set_angvel(angular_velocity, true);
        }
    }

    /// Get the angular velocity of a rigid body (rotation speed in radians per second).
    pub fn get_angular_velocity(&self, body_handle: RigidBodyHandle) -> Option<f32> {
        self.physics_world.get(body_handle).map(|body| body.angvel())
    }

    /// Set linear damping (resistance to movement, 0.0 = no damping, higher = more resistance).
    pub fn set_linear_damping(&mut self, body_handle: RigidBodyHandle, damping: f32) {
        if let Some(body) = self.physics_world.get_mut(body_handle) {
            body.set_linear_damping(damping);
        }
    }

    /// Set angular damping (resistance to rotation, 0.0 = no damping, higher = more resistance).
    pub fn set_angular_damping(&mut self, body_handle: RigidBodyHandle, damping: f32) {
        if let Some(body) = self.physics_world.get_mut(body_handle) {
            body.set_angular_damping(damping);
        }
    }

    /// Get the rigid body handle for an entity, if it exists.
    pub fn get_body_handle(&self, entity: EntityId) -> Option<RigidBodyHandle> {
        self.entity_to_body.get(&entity).copied()
    }

    /// Get the entity ID for a rigid body handle, if it exists.
    pub fn get_entity(&self, body_handle: RigidBodyHandle) -> Option<EntityId> {
        self.body_to_entity.get(&body_handle).copied()
    }

    /// Remove a rigid body and all its colliders.
    pub fn remove_body(&mut self, entity: EntityId) -> bool {
        if let Some(handle) = self.entity_to_body.remove(&entity) {
            self.physics_world.remove(
                handle,
                &mut self.island_manager,
                &mut self.collider_set,
                &mut self.impulse_joint_set,
                &mut self.multibody_joint_set,
                true,
            );
            self.body_to_entity.remove(&handle);
            true
        } else {
            false
        }
    }

    /// Cast a ray and return the first hit, if any.
    pub fn cast_ray(
        &self,
        origin: Vec2,
        direction: Vec2,
        max_toi: f32,
    ) -> Option<(EntityId, Vec2, f32)> {
        let ray = Ray::new(
            point![origin.x, origin.y],
            vector![direction.x, direction.y],
        );

        if let Some((handle, toi)) = self.query_pipeline.cast_ray(
            &self.physics_world,
            &self.collider_set,
            &ray,
            max_toi,
            true,
            QueryFilter::default(),
        ) {
            if let Some(collider) = self.collider_set.get(handle) {
                if let Some(body_handle) = collider.parent() {
                    if let Some(entity) = self.get_entity(body_handle) {
                        let hit_point = ray.point_at(toi);
                        return Some((entity, Vec2::new(hit_point.x, hit_point.y), toi));
                    }
                }
            }
        }
        None
    }

    /// Check if a point is inside any collider.
    pub fn point_query(&self, point: Vec2) -> Option<EntityId> {
        let point = point![point.x, point.y];
        // Use the collider set directly for point queries
        let mut found_entity = None;
        for (_handle, collider) in self.collider_set.iter() {
            if let Some(body_handle) = collider.parent() {
                let shape = collider.shape();
                if shape.contains_point(
                    collider.position(),
                    &point,
                ) {
                    if let Some(entity) = self.get_entity(body_handle) {
                        found_entity = Some(entity);
                        break;
                    }
                }
            }
        }
        found_entity
    }
}

impl Default for PhysicsWorld {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper functions for creating common collider shapes.
impl PhysicsWorld {
    /// Create a box (rectangle) collider shape.
    pub fn create_box_collider(half_width: f32, half_height: f32) -> SharedShape {
        SharedShape::cuboid(half_width, half_height)
    }

    /// Create a circle collider shape.
    pub fn create_circle_collider(radius: f32) -> SharedShape {
        SharedShape::ball(radius)
    }

    /// Create a capsule (pill) collider shape.
    pub fn create_capsule_collider(half_height: f32, radius: f32) -> SharedShape {
        SharedShape::capsule_y(half_height, radius)
    }
}

impl PhysicsWorld {
    /// Access the impulse joint set for advanced joint creation.
    ///
    /// This allows you to create joints using Rapier2D's joint API directly.
    /// Example:
    /// ```rust,no_run
    /// use rapier2d::prelude::*;
    /// let joint = RevoluteJoint::new(point![0.0, 0.0], point![0.0, 0.0]);
    /// physics.impulse_joint_set_mut().insert(body1, body2, joint, true);
    /// ```
    pub fn impulse_joint_set_mut(&mut self) -> &mut ImpulseJointSet {
        &mut self.impulse_joint_set
    }

    /// Remove a joint.
    pub fn remove_joint(&mut self, joint_handle: ImpulseJointHandle) {
        self.impulse_joint_set.remove(joint_handle, true);
    }
}

/// Re-export commonly used Rapier types for convenience.
pub use rapier2d::prelude::{ImpulseJointHandle, RigidBodyType};

