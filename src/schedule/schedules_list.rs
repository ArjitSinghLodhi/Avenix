use crate::{
    schedule::{ScheduleLabel, SystemExecutor, SystemsSchedule},
    world::storage::World,
};

#[derive(Debug)]
pub struct Startup;

impl ScheduleLabel for Startup {
    fn default_executor(&mut self) -> Box<dyn SystemExecutor> {
        Box::new(StartupExecutor)
    }
}

pub(crate) struct StartupExecutor;

impl SystemExecutor for StartupExecutor {
    fn run(&mut self, schedule: &mut SystemsSchedule, world: &mut World) {
        for system in schedule.systems_mut() {
            let should_run = system.run_conditions.iter().all(|cond| cond(world));
            if should_run {
                system.run(world);
            }
            world.apply_commands();
        }
    }
}

pub struct First;

impl ScheduleLabel for First {}

pub struct PreUpdate;

impl ScheduleLabel for PreUpdate {}

pub struct Update;

impl ScheduleLabel for Update {}

pub struct PostUpdate;

impl ScheduleLabel for PostUpdate {}

/// A special schedule where queue commands are applied immediately before
/// and after the systems registered in this schedule run. Despawn commands are not applied yet.
///
/// See [`DefaultSchedulesPlugin`] for more information on the execution order.
///
/// [`DefaultSchedulesPlugin`]: crate::schedule::DefaultSchedulesPlugin
pub struct CleanupHandles;

impl ScheduleLabel for CleanupHandles {
    fn default_executor(&mut self) -> Box<dyn SystemExecutor> {
        Box::new(CleanupHandlesExecutor)
    }
}

pub(crate) struct CleanupHandlesExecutor;

impl SystemExecutor for CleanupHandlesExecutor {
    fn run(&mut self, schedule: &mut SystemsSchedule, world: &mut World) {
        world.apply_queue_commands();
        for system in schedule.systems_mut() {
            let should_run = system.run_conditions.iter().all(|cond| cond(world));
            if should_run {
                system.run(world);
            }
        }
        world.apply_queue_commands();
    }
}

/// A special schedule where systems registered in this schedule run,
/// followed by the execution of queue commands, and finally, despawn commands are processed.
///
/// See [`DefaultSchedulesPlugin`] for more information on the execution order.
///
/// [`DefaultSchedulesPlugin`]: crate::schedule::DefaultSchedulesPlugin
pub struct ApplyCommands;

impl ScheduleLabel for ApplyCommands {
    fn default_executor(&mut self) -> Box<dyn SystemExecutor> {
        Box::new(ApplyCommandsExecutor)
    }
}

pub struct ApplyCommandsExecutor;

impl SystemExecutor for ApplyCommandsExecutor {
    fn run(&mut self, schedule: &mut SystemsSchedule, world: &mut World) {
        for system in schedule.systems_mut() {
            let should_run = system.run_conditions.iter().all(|cond| cond(world));
            if should_run {
                system.run(world);
            }
        }
        world.apply_queue_commands();
        world.apply_despawns();
    }
}

pub struct Last;

impl ScheduleLabel for Last {}
