use crate::app::{App, Plugin};

mod schedules_list;

pub use schedules_list::{
    ApplyCommands, CleanupHandles, First, Last, PostUpdate, PreUpdate, Startup, Update,
};

use crate::extensions::{System, World};

use std::{
    any::{Any, TypeId},
    fmt::Debug,
};

/// A plugin that registers the default execution schedules:
/// `First`, `PreUpdate`, `Update`, `PostUpdate`, `CleanupHandles`, `ApplyCommands`, and `Last`.
///
/// Among these, the `CleanupHandles` and `ApplyCommands` schedules serve special purposes:
///
/// * **`CleanupHandles`**: Queue commands (such as spawning and adding components) are applied
///   immediately before and after the systems registered in this schedule run. Despawn commands
///   are **not** applied yet.
/// * **`ApplyCommands`**: Systems in this schedule run, followed by the execution of
///   queue commands, and finally, despawn commands are processed.
///
/// This sequence is a deliberate mechanism designed to facilitate the use of the
/// [`Commands::despawn_iter()`] and [`Commands::will_despawn()`] functions. It ensures adherence
/// to Avenix's strict rule: *All cloned handles referencing an entity must be dropped before
/// the entity's despawn command is applied.*
///
/// [`Commands::despawn_iter()`]: crate::commands::Commands::despawn_iter
/// [`Commands::will_despawn()`]: crate::commands::Commands::will_despawn
pub struct DefaultSchedulesPlugin;

impl Plugin for DefaultSchedulesPlugin {
    fn build(self, app: &mut App) {
        app.add_schedule(First)
            .add_schedule(PreUpdate)
            .add_schedule(Update)
            .add_schedule(PostUpdate)
            .add_schedule(CleanupHandles)
            .add_schedule(ApplyCommands)
            .add_schedule(Last);

        app.configure_schedule_order(First, PreUpdate)
            .configure_schedule_order(PreUpdate, Update)
            .configure_schedule_order(Update, PostUpdate)
            .configure_schedule_order(PostUpdate, CleanupHandles)
            .configure_schedule_order(CleanupHandles, ApplyCommands)
            .configure_schedule_order(ApplyCommands, Last);
    }
}

#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScheduleId {
    pub(crate) id: TypeId,
    pub(crate) name: &'static str,
}

#[doc(hidden)]
pub trait IntoScheduleId: ScheduleLabel {
    fn id(&self) -> ScheduleId
    where
        Self: Sized,
    {
        ScheduleId {
            id: TypeId::of::<Self>(),
            name: std::any::type_name::<Self>(),
        }
    }
}

impl<T: ?Sized + ScheduleLabel> IntoScheduleId for T {}

#[doc(hidden)]
pub trait ScheduleLabel: Any + Send + Sync {
    fn default_executor(&mut self) -> Box<dyn SystemExecutor> {
        Box::new(SingleThreadedExecutor)
    }
}
type ConditionFn = Box<dyn Fn(&World) -> bool + Send + Sync>;

#[doc(hidden)]
pub struct SystemNode {
    system: Box<dyn System>,
    pub(crate) run_conditions: Vec<ConditionFn>,
}

impl SystemNode {
    pub fn new(system: Box<dyn System>) -> Self {
        Self {
            system,
            run_conditions: Vec::new(),
        }
    }

    pub fn run(&mut self, world: &mut World) {
        self.system.run(world);
    }
}

#[doc(hidden)]
pub struct SystemsSchedule {
    systems: Vec<SystemNode>,
}

impl SystemsSchedule {
    pub(crate) fn new() -> Self {
        Self {
            systems: Vec::new(),
        }
    }

    pub fn systems_mut(&mut self) -> impl Iterator<Item = &mut SystemNode> {
        self.systems.iter_mut()
    }
}

#[doc(hidden)]
pub trait SystemExecutor: Send + Sync {
    fn run(&mut self, schedule: &mut SystemsSchedule, world: &mut World);
}

#[doc(hidden)]
#[derive(Default)]
pub struct SingleThreadedExecutor;

impl SystemExecutor for SingleThreadedExecutor {
    fn run(&mut self, schedule: &mut SystemsSchedule, world: &mut World) {
        for node in schedule.systems_mut() {
            let should_run = node.run_conditions.iter().all(|cond| cond(world));

            if should_run {
                node.system.run(world);
            }
        }
    }
}

#[doc(hidden)]
pub struct Schedule {
    id: ScheduleId,
    systems_schedule: SystemsSchedule,
    executor: Box<dyn SystemExecutor>,
}

impl Schedule {
    pub fn new<L: ScheduleLabel + 'static>(mut label: L) -> Self {
        Self {
            id: label.id(),
            executor: label.default_executor(),
            systems_schedule: SystemsSchedule::new(),
        }
    }

    pub fn id(&self) -> ScheduleId {
        self.id
    }

    pub fn set_executor(&mut self, executor: impl SystemExecutor + 'static) {
        self.executor = Box::new(executor);
    }

    pub fn add_system(&mut self, system: Box<dyn System>) {
        self.systems_schedule.systems.push(SystemNode::new(system));
    }

    pub fn run(&mut self, world: &mut World) {
        self.executor.run(&mut self.systems_schedule, world);
    }
}
