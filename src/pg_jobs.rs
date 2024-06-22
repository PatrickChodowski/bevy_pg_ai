use bevy::app::{App, Plugin, PreUpdate, Update, Startup};
use bevy::asset::{Asset, AssetServer, Assets, LoadedFolder, RecursiveDependencyLoadState, Handle};
use bevy::ecs::schedule::common_conditions::{on_event, resource_changed, resource_exists};
use bevy::ecs::schedule::{IntoSystemConfigs, Condition};
use bevy::ecs::entity::Entity;
use bevy::ecs::event::{Event, EventReader};
use bevy::ecs::component::Component;
use bevy::ecs::system::{Commands, Local, Resource, Res, ResMut, Query};
use bevy::ecs::query::With;
use bevy::sprite::Anchor;
use bevy::hierarchy::DespawnRecursiveExt;
use bevy::transform::components::Transform;
use bevy::utils::default;
use bevy::hierarchy::{BuildChildren, Parent};
use bevy::reflect::TypePath;
use bevy::render::color::Color;
use bevy::text::{Text2dBundle, TextStyle, Text};
use bevy_common_assets::json::JsonAssetPlugin;
use bevy_common_assets::toml::TomlAssetPlugin;
use bevy_pg_calendar::prelude::{Calendar, CalendarNewHourEvent, Cron};
use serde::{Deserialize, Serialize};

use crate::pg_tasks::JobTasks;


pub struct PGJobsPlugin {
    pub active: bool,
    pub debug:  bool
}
impl Default for PGJobsPlugin {
    fn default() -> Self {
        PGJobsPlugin{
            active:        true,
            debug:         true
        }
    }
}

impl Plugin for PGJobsPlugin {
    fn build(&self, app: &mut App) {
        app
        .add_plugins(JsonAssetPlugin::<Job>::new(&["job.json"]))
        .add_plugins(TomlAssetPlugin::<Job>::new(&["job.toml"]))
        .insert_resource(Jobs::init(self.active, self.debug))
        .insert_resource(JobCatalog::init())
        .add_event::<StopJobEvent>()
        .add_event::<StartJobEvent>()
        .add_systems(Startup,   init)
        .add_systems(Update,    track.run_if(resource_exists::<LoadedJobsHandles>))
        .add_systems(PreUpdate, (trigger_jobs_calendar.run_if(on_event::<CalendarNewHourEvent>()), 
                                 trigger_jobs_time)
                                .chain()
                                .run_if(if_jobs_active))
        .add_systems(Update,    debug_jobs.run_if(if_jobs_debug.and_then(resource_changed::<Jobs>)))
        .add_systems(PreUpdate, (stop_job.run_if(on_event::<StopJobEvent>()), 
                                 start_job.run_if(on_event::<StartJobEvent>())).chain())
        // .add_systems(Update,    handle_folder_jobs    update_fail_jobs.run_if(if_active)
        //                                         .after(init_jobs))
        ;
    }
}

#[derive(Event)]
pub struct StopJobEvent {
    pub entity: Entity
}

#[derive(Event)]
pub struct StartJobEvent {
    pub job_id: u32,
    pub entity: Entity
}

#[derive(Resource)]
pub struct LoadedJobsHandles(Handle<LoadedFolder>);

// Read in all jobs from data files into asset server

fn init(mut commands:   Commands,
        ass:            Res<AssetServer>, ){
    let handle_folder_jobs: Handle<LoadedFolder> = ass.load_folder("jobs/");
    commands.insert_resource(LoadedJobsHandles(handle_folder_jobs));
}

struct JobsReady(bool);
impl Default for JobsReady {
    fn default() -> Self {
        JobsReady(false)
    }
}


fn track(
    mut commands:      Commands,
    ass:               Res<AssetServer>,
    mut job_ready:     Local<JobsReady>, 
    mut ass_jobs:      ResMut<Assets<Job>>,
    mut jobs_catalog:  ResMut<JobCatalog>,
    loaded_data:       Res<LoadedJobsHandles>
){

    if !job_ready.0 {
    if let Some(scenes_load_state) = ass.get_recursive_dependency_load_state(&loaded_data.0) {
        if scenes_load_state == RecursiveDependencyLoadState::Loaded {
            job_ready.0 = true;
        }
    }
    }

    if job_ready.0 {

    for (_job_id, job) in ass_jobs.iter_mut(){
        job.schedule.parse();
        jobs_catalog.add(job.clone());
    }

    commands.remove_resource::<LoadedJobsHandles>();

    }

}

pub fn if_jobs_active(jobs: Res<Jobs>) -> bool {
    jobs.active
}

pub fn if_jobs_debug(jobs: Res<Jobs>) -> bool {
    jobs.debug
}


#[derive(Resource)]
pub struct JobCatalog {
    pub data: Vec<Job>
}
impl JobCatalog {
    pub fn init() -> Self {
        JobCatalog { data: Vec::new() }
    }
    pub fn add(&mut self, job: Job) {
        self.data.push(job);
    }
    pub fn clear(&mut self){
        self.data.clear();
    } 
    pub fn get(&self, id: u32) -> Option<Job> {
        for job in self.data.iter() {
            if job.id == id {
                return Some(job.clone());
            }
        }
        return None;
    }
}

#[derive(Resource)]
pub struct Jobs {
    active: bool,                   // For all jobs
    debug:  bool,
    data:   Vec<Job>
}
impl Jobs {
    fn init(active: bool, debug: bool) -> Self {
        Self {active, debug, data: Vec::new()}
    }
    pub fn add(&mut self, job: Job) {
        self.data.push(job); // This allows for multiple jobs per entity :o
    }

    pub fn get(&self, entity: &Entity) -> Option<&Job> {
        for job in self.data.iter(){
            if let Some(job_entity) = job.entity {
                if entity == &job_entity {
                    return Some(job);
                }
            }
        }
        return None;
    }

    pub fn get_mut(&mut self, entity: &Entity) -> Option<&mut Job> {
        for job in self.data.iter_mut(){
            if let Some(job_entity) = job.entity {
                if entity == &job_entity {
                    return Some(job);
                }
            }
        }
        return None;
    }

    pub fn next_task(&mut self, commands: &mut Commands, task_entity: &Entity) {
        if let Some(job) = self.get_mut(&task_entity) {
            let next_task_type = job.tasks.next_task();
            next_task_type.add_task(commands, task_entity);
        } else {
            panic!("no entity {:?} in jobs", task_entity);
        }
    }
    pub fn jump_task(&mut self, commands: &mut Commands, task_entity: &Entity, next_task_id: u32){
        if let Some(job) = self.get_mut(&task_entity) {
            let next_task_type = job.tasks.set_task(next_task_id);
            next_task_type.add_task(commands, task_entity);
        } else {
            panic!("no entity {:?} in jobs", task_entity);
        }
    }
    pub fn index(&self, entity: Entity) -> Option<usize> {
        return self.data.iter().position(|x| x.entity == Some(entity));
    }
    pub fn upsert(&mut self, entity: Entity, job: Job) {
        if let Some(index) = self.index(entity){
            self.data[index] = job;
        } else {
            self.data.push(job);
        }
    }
    pub fn remove(&mut self, job_id: u32, entity: &Entity) {
        self.data.retain(|x| !(x.entity == Some(*entity) && x.id == job_id))
    }
    pub fn remove_all(&mut self, entity: &Entity) {
        self.data.retain(|x| x.entity != Some(*entity))
    }
    pub fn activate(&mut self) {
        self.active = true;
    }
    pub fn deactivate(&mut self) {
        self.active = false;
    }
    pub fn clear(&mut self) {
        self.data.clear();
    }
    pub fn set_debug(&mut self, b: bool) {
        self.debug = b
    }
    pub fn get_debug(&self) -> bool {
        self.debug
    }
    pub fn get_data(&self) -> &Vec<Job> {
        &self.data
    }
}

#[derive(PartialEq, Copy, Clone, Serialize, Deserialize, Debug)]
pub enum JobStatus {
    ToDo,
    Active,
    Done
}

#[derive(Serialize, Deserialize, Asset, TypePath, Clone, Debug)]
pub struct Job {
    pub entity:        Option<Entity>,    // In the beginning there might not be entity.
    pub status:        JobStatus,
    pub schedule:      JobSchedule,       // Schedule that will start the Job
    pub tasks:         JobTasks,          // List of job.set_active();tasks to be performed by entity
    pub id:            u32,               // Unique job id to search from catalog
    pub loopk:         u32,               // Used for loops to count iterations
    pub fail_task_id:  u32,               // ID of task to perform if task failed
    pub fail_job_id:   u32,               // ID of task to perform if job failed to start 
    pub active:        bool,              // Toggle to activate/deactivate single task
}

impl Default for Job {
    fn default() -> Self {
        Job{
            entity:         None,
            status:         JobStatus::ToDo,
            schedule:       JobSchedule::Instant, 
            tasks:          JobTasks::new(),
            id:             0,
            loopk:          0,
            fail_task_id:   0,
            fail_job_id:    0,
            active:         true,
        }
    }
}

impl Job {
    pub fn start(
        &mut self, 
        commands: &mut Commands
    ){
        let entity = self.tasks.start(commands, self.entity);
        if self.entity.is_none(){
            self.entity = Some(entity);
        }
        self.set_active();
    }

    pub fn new() -> Self {
        let job = Job::default();
        return job;
    }
    pub fn get_status(&mut self) -> JobStatus {
        self.status
    }
    pub fn set_active(&mut self){
        self.status = JobStatus::Active;
    }
    pub fn set_done(&mut self){
        self.status = JobStatus::Done;
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum JobSchedule {      
    Instant,             // Start instantly       
    Cron(Cron),          // Waiting for Cron 
    Delay(u8),           // Delay in in-game hours
    RealDelay(f32)       // Real time delay  
} 
impl JobSchedule {
    pub fn parse(&mut self) {
        match self {
            JobSchedule::Cron(ref mut cron) => {cron.parse()}
            _ => {}
        }
    }
}


// Update jobs. Triggers every hour from calendar.
fn trigger_jobs_calendar(
    mut commands:         Commands,
    calendar:             Res<Calendar>,
    mut jobs:             ResMut<Jobs>
){

    for job in jobs.data.iter_mut(){

        if !job.active {
            continue;
        }

        if job.get_status() != JobStatus::ToDo {
            continue;
        }

        match &job.schedule {
            JobSchedule::Cron(cron) => {
                if cron.hours.as_ref().unwrap().contains(&calendar.get_current_hour()) && 
                   cron.days_week.as_ref().unwrap().contains(&calendar.get_current_weekday()){
                    job.start(&mut commands);
                }
             }
            _=> {}
        }
    }

}

// Updates jobs on real time
fn trigger_jobs_time(
    mut commands:           Commands,
    mut jobs:               ResMut<Jobs>
){

    for job in jobs.data.iter_mut(){

        if !job.active {
            continue;
        }
        if job.get_status() != JobStatus::ToDo {
            continue;
        }

        match &job.schedule {
            JobSchedule::Instant => {
                job.start(&mut commands);
            }
            _=> {}
        }
    }
}

fn stop_job(
    mut commands:       Commands,
    mut jobs:           ResMut<Jobs>,
    mut stop_job:       EventReader<StopJobEvent>,
    jobdebugs:          Query<(Entity, &Parent), With<JobDebug>>
){
    for ev in stop_job.read(){
        if let Some(job) = jobs.get(&ev.entity){
            let task = job.tasks.get_current();
            task.remove(&mut commands, ev.entity);
        }
        jobs.remove_all(&ev.entity);

        for (text_entity, task_entity) in jobdebugs.iter(){
            if **task_entity == ev.entity {
                commands.entity(text_entity).despawn_recursive();
                break;
            }
        }
    }
}

fn start_job(
    mut jobs:           ResMut<Jobs>,
    jobs_catalog:       Res<JobCatalog>,
    mut start_job:      EventReader<StartJobEvent>
){
    for ev in start_job.read(){
        if let Some(mut job) = jobs_catalog.get(ev.job_id) {
            job.entity = Some(ev.entity);
            jobs.add(job);
        }
    }
}


#[derive(Component)]
struct JobDebug;



// Displays each entity current task and its parameters
fn debug_jobs(
    mut commands:     Commands, 
    ass:              Res<AssetServer>,
    jobs:             Res<Jobs>, 
    mut jobdebugs:    Query<(Entity, &Parent, &mut Text), With<JobDebug>>
){
    
    let font = ass.load("fonts/FiraMono-Medium.ttf");
    let text_style = TextStyle {
        font: font.clone(),
        font_size: 20.0,
        color: Color::WHITE,
    };

    if jobs.get_debug(){
        for job in jobs.data.iter(){
            if let Some(job_entity) = job.entity {
    
                let current_task = job.tasks.get_current();
                let mut needs_label: bool = true;
    
                for(_text_entity, text_parent, mut text) in jobdebugs.iter_mut(){
    
                    if **text_parent != job_entity {
                        continue; 
                    }
                    needs_label = false;
                    text.sections[0].value = current_task.display();
                } 
    
    
                if needs_label {
                    let debug_text = commands.spawn((
                        Text2dBundle {
                            text: Text::from_section(current_task.display(), text_style.clone()),
                            text_anchor: Anchor::TopCenter,
                            transform: Transform::from_xyz(0.0, 70.0, 10.0),
                            ..default()
                        },
                        JobDebug,
                    )).id();
                
                    commands.entity(job_entity).add_child(debug_text);
                }
    
            }
    
        }
    } else {
        for(text_entity, _text_parent, _text) in jobdebugs.iter(){
            commands.entity(text_entity).despawn();
        }
    }
}
