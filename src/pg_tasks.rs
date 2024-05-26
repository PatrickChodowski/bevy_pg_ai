// Bevy dependencies
use bevy::prelude::*;
use bevy::utils::HashMap;
use bevy::sprite::MaterialMesh2dBundle;

use libm::{atan2f, fabsf, cosf, sinf}; 

// Crate dependencies
use bevy_pg_calendar::prelude::{Calendar, CalendarNewHourEvent};
use crate::pg_jobs::Jobs;
use crate::prelude::JobSchedule;

pub const SPAWN_TASK_ID:   u32 = 0;
pub const DESPAWN_TASK_ID: u32 = 1000;

pub struct TasksPlugin;

impl Plugin for TasksPlugin {
    fn build(&self, app: &mut App) {
        app
        .add_systems(Update, (spawn_task, 
                              wait_task_time, 
                              wait_idle_calendar.run_if(on_event::<CalendarNewHourEvent>()),
                              move_task,
                              rotate_task,
                              teleport_task,
                              hide_task,
                              show_task,
                              decision_task,
                              despawn_task
                            ))
        ;
    }
}

// WaitForEvent     Task
// SendEvent        Task
// InsertComponent  Task
// LoopNKTask
// LoopNTask
// Decision task


// Fill it up with the tasks for the game
#[derive(Component, Clone)]
pub enum TaskType {
    Spawn(SpawnTask),
    Despawn(DespawnTask),
    Move(MoveTask),
    Rotate(RotateTask),
    Wait(WaitTask),
    Hide(HideTask),
    Show(ShowTask),
    Teleport(TeleportTask),
    Decision(DecisionTask)
}
impl TaskType {
    pub fn spawn_with_task(&self, commands: &mut Commands) -> Entity {
        match &self {
            TaskType::Spawn(data)       => {commands.spawn(*data).id()}
            TaskType::Despawn(data)     => {commands.spawn(*data).id()}
            TaskType::Move(data)        => {commands.spawn(*data).id()}
            TaskType::Rotate(data)      => {commands.spawn(*data).id()}
            TaskType::Wait(data)        => {commands.spawn(data.clone()).id()}
            TaskType::Hide(data)        => {commands.spawn(*data).id()}
            TaskType::Show(data)        => {commands.spawn(*data).id()}
            TaskType::Teleport(data)    => {commands.spawn(*data).id()}
            TaskType::Decision(data)    => {commands.spawn(*data).id()}
        }
    }
    pub fn add_task(&self, commands: &mut Commands, entity: &Entity) {
        match &self {
            TaskType::Spawn(data)    => {commands.entity(*entity).insert(*data);}
            TaskType::Despawn(data)  => {commands.entity(*entity).insert(*data);}
            TaskType::Move(data)     => {commands.entity(*entity).insert(*data);}
            TaskType::Rotate(data)   => {commands.entity(*entity).insert(*data);}
            TaskType::Wait(data)     => {commands.entity(*entity).insert(data.clone());}
            TaskType::Hide(data)     => {commands.entity(*entity).insert(*data);}
            TaskType::Show(data)     => {commands.entity(*entity).insert(*data);}
            TaskType::Teleport(data) => {commands.entity(*entity).insert(*data);}
            TaskType::Decision(data) => {commands.entity(*entity).insert(*data);}
        }
    }
}



// JobTasks
pub struct JobTasks {
    pub data:                   HashMap<u32, TaskType>,   
    pub statuses:               HashMap<u32, TaskStatus>,  // Statues of the tasks 
    pub current_task_id:        u32,
}

impl JobTasks {
    pub fn new() -> Self {
        JobTasks{
            data:                   HashMap::new(),
            statuses:               HashMap::new(),
            current_task_id:        0,
        }
    }

    pub fn add(&mut self, id: u32, task: TaskType) {
        self.data.insert(id, task);

        if id == SPAWN_TASK_ID {
            self.statuses.insert(id, TaskStatus::ToDo);
        } else {
            self.statuses.insert(id, TaskStatus::Waiting);
        }
    }

    pub fn start(&mut self, commands: &mut Commands) -> Entity {
        let current_task = &self.data[&self.current_task_id];
        let entity = current_task.spawn_with_task(commands);
        self.set_current_status(TaskStatus::Active);
        return entity;
    }

    pub fn next_task(&mut self) -> &TaskType {
        match self.get_current_status() {
            &TaskStatus::Done => {
                // Should be only if loop was requested to close
                self.current_task_id += 1;
                self.set_current_status(TaskStatus::ToDo);
                return self.get_current();
            }
            &TaskStatus::Active => {
                self.set_current_status(TaskStatus::Done);
                self.current_task_id += 1;
                self.set_current_status(TaskStatus::ToDo);
                return self.get_current();
            }
            &TaskStatus::ToDo => {
                // When the loop task finished
                self.set_current_status(TaskStatus::Done);
                self.current_task_id += 1;
                self.set_current_status(TaskStatus::ToDo);
                return self.get_current();
            }
            _ => {
                panic!("Not supposed to happen {:?}", self.current_task_id )
            }
        }
    }

    pub fn get_current(&self) -> &TaskType {
        &self.data[&self.current_task_id]
    }
    pub fn set_current_status(&mut self, status: TaskStatus) {
        self.statuses.insert(self.current_task_id, status);
    }
    pub fn get_current_status(&mut self) -> &TaskStatus {
        self.statuses.get(&self.current_task_id).unwrap()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TaskStatus {
    ToDo,
    Waiting,
    Active,
    // InLoop(usize), // Number of task steps to repeat every time
    Done,
    Fail
}


// Define component for each task

#[derive(Component, Clone, Copy)]
pub struct SpawnTask {
    pub color:  Color,
    pub loc:    Vec3
}

#[derive(Component, Clone, Copy)]
pub struct DespawnTask;

#[derive(Component, Clone, Copy)]
pub struct MoveTask {
    pub source:         Vec3,
    pub target:         Vec3,
}

#[derive(Component, Clone, Copy)]
pub struct RotateTask {
    pub angle:      f32
}

#[derive(Component, Clone)]
pub struct WaitTask {
    pub schedule: JobSchedule
}

#[derive(Component, Clone, Copy)]
pub struct HideTask;

#[derive(Component, Clone, Copy)]
pub struct ShowTask;

#[derive(Component, Clone, Copy)]
pub struct TeleportTask {
    pub loc: Vec3
}

#[derive(Component, Clone, Copy)]
pub struct DecisionTask {
    pub opt1: u32,
    pub opt2: u32
}

// Task systems

fn spawn_task(
    mut commands:   Commands,
    mut jobs:       ResMut<Jobs>,
    tasks:          Query<(Entity, &SpawnTask)>,
    mut meshes:     ResMut<Assets<Mesh>>,
    mut materials:  ResMut<Assets<ColorMaterial>>,
){

    for (task_entity, spawn_task) in tasks.iter(){

        commands.entity(task_entity).insert(
            MaterialMesh2dBundle {
                mesh: meshes.add(Rectangle::from_size(Vec2 { x: 100.0, y: 100.0 })).into(),
                transform: Transform::from_translation(spawn_task.loc),
                material: materials.add(spawn_task.color),
                ..default()}
        );

        commands.entity(task_entity).remove::<SpawnTask>();
        jobs.next_task(&mut commands, &task_entity);
    }
}


fn wait_task_time(mut commands:   Commands,
                  mut jobs:       ResMut<Jobs>,
                  time:           Res<Time>,
                  mut tasks:      Query<(Entity, &mut WaitTask)>,){

    for (task_entity, mut wait_task) in tasks.iter_mut(){

        match &mut wait_task.schedule {
            JobSchedule::RealDelay(delay) => {
                if *delay > 0.0 {
                    *delay -= time.delta_seconds();
                } else {
                    commands.entity(task_entity).remove::<WaitTask>();
                    jobs.next_task(&mut commands, &task_entity);
                }
            }
            _ => {}
        }
    }
}


pub fn wait_idle_calendar(mut commands:     Commands,
                          mut jobs:         ResMut<Jobs>,
                          calendar:         Res<Calendar>,
                          mut idle_cars:    Query<(Entity, &mut WaitTask)>){

    for (task_entity, mut wait_task) in idle_cars.iter_mut(){
        match &mut wait_task.schedule {
                JobSchedule::Cron(cron) => {
                    if cron.hours.as_ref().unwrap().contains(&calendar.get_current_hour()) && 
                       cron.days_week.as_ref().unwrap().contains(&calendar.get_current_weekday()){

                        commands.entity(task_entity).remove::<WaitTask>();
                        jobs.next_task(&mut commands, &task_entity);

                    }
                 }
                 JobSchedule::Delay(delay) => {
                    if *delay > 0 {
                        *delay -= 1;
                    } else {
                        commands.entity(task_entity).remove::<WaitTask>();
                        jobs.next_task(&mut commands, &task_entity);
                    }
                }
                _=> {}   
        }
    }

}

fn move_task(mut commands:   Commands,
             mut jobs:       ResMut<Jobs>,
             time:           Res<Time>,
             mut tasks:      Query<(Entity, &mut Transform, &mut MoveTask)>,){

    let speed = 200.0;
    for (task_entity, mut transform, move_task) in tasks.iter_mut(){

        let angle: f32 = get_direction(&transform.translation.xy(), &move_task.target.xy());
        let dist: f32 = get_distance_manhattan(&transform.translation.xy(), &move_task.target.xy());
        let local_speed = speed*time.delta_seconds();
        if local_speed > dist {
            commands.entity(task_entity).remove::<MoveTask>();
            jobs.next_task(&mut commands, &task_entity);
        } else {
            // transform.look_at(move_task.target, Vec3::Z);
            transform.translation.x += local_speed * cosf(angle);
            transform.translation.y += local_speed * sinf(angle);     
        }

    }

}


pub fn get_direction(source_xy: &Vec2, target_xy: &Vec2) -> f32 {
    return atan2f(target_xy.y - source_xy.y, target_xy.x - source_xy.x);
}

pub fn get_distance_manhattan(source: &Vec2, target: &Vec2) -> f32 {
    return fabsf(target.x - source.x) + fabsf(target.y - source.y);
}

fn rotate_task(mut commands:   Commands,
               mut jobs:       ResMut<Jobs>,
               mut tasks:      Query<(Entity, &mut Transform, &RotateTask)>,){

    let ang_speed = 0.05;
    for (task_entity, mut transform, rotate_task) in tasks.iter_mut(){
        let angle: f32 = transform.rotation.to_euler(EulerRot::XYZ).2.to_degrees();
        if angle < rotate_task.angle {
            transform.rotate_z(ang_speed);
        } else {
            commands.entity(task_entity).remove::<RotateTask>();
            jobs.next_task(&mut commands, &task_entity);
        }
    }
}

fn teleport_task(mut commands:   Commands,
                 mut jobs:       ResMut<Jobs>,
                 mut tasks:      Query<(Entity, &mut Transform, &TeleportTask)>){

    for (task_entity, mut transform, teleport_task) in tasks.iter_mut(){
        transform.translation = teleport_task.loc;
        commands.entity(task_entity).remove::<TeleportTask>();
        jobs.next_task(&mut commands, &task_entity);
    }
    
}

fn show_task(mut commands:   Commands,
             mut jobs:       ResMut<Jobs>,
             mut tasks:      Query<(Entity, &mut Visibility), With<ShowTask>>){

    for (task_entity, mut vis) in tasks.iter_mut(){
        *vis = Visibility::Inherited;
        commands.entity(task_entity).remove::<ShowTask>();
        jobs.next_task(&mut commands, &task_entity);
    }
}

fn hide_task(mut commands:   Commands,
             mut jobs:       ResMut<Jobs>,
             mut tasks:      Query<(Entity, &mut Visibility), With<HideTask>>){

    for (task_entity, mut vis) in tasks.iter_mut(){
        *vis = Visibility::Hidden;
        commands.entity(task_entity).remove::<HideTask>();
        jobs.next_task(&mut commands, &task_entity);
    }
}

// Example Decision task
fn decision_task(mut commands:   Commands,
                 mut jobs:       ResMut<Jobs>,
                 tasks:          Query<(Entity, &DecisionTask)>){

    for (task_entity, decision_task) in tasks.iter(){

    }

}



fn despawn_task(
    mut commands:   Commands,
    mut jobs:       ResMut<Jobs>,
    tasks:          Query<Entity, With<DespawnTask>>){

    for task_entity in tasks.iter(){
        commands.entity(task_entity).despawn_recursive();
        jobs.remove(&task_entity);
    }

}

