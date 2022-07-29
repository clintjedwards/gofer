use std::collections::HashMap;

use gofer_proto::trigger::{TriggerState, TriggerStatus};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

/// TriggerResultState is a description of an events specific outcome. Events are normally generated by triggers
/// so its helpful to have them be able to pass down an event, but also give an indication on what that event
/// might mean.
///
/// For example: A trigger that evaluates whether a pipeline should run on a specific date might also skip certain
/// holidays. In this case it would pass down an "skipped" event result to inform the user that their pipeline
/// would have ran, but did not due to holiday.
pub enum Result {
    Unknown,
    Success,
    Failure,
    Skipped,
}

/// Since triggers are run solely via containers, we need a way to track their state so that Gofer understands
/// what a container is currently doing and when it's ready to serve traffic.
#[derive(Debug, Clone, Display, EnumString, Serialize, Deserialize, PartialEq, Eq)]
pub enum State {
    /// Cannot determine state of Trigger, should never be in this state.
    Unknown,
    /// Going through pre-scheduling verification and prep.
    Processing,
    /// Running and ready to process requests.
    Running,
    /// Trigger has exited; usually because of an error.
    Exited,
}

impl From<TriggerState> for State {
    fn from(r: TriggerState) -> Self {
        match r {
            TriggerState::UnknownState => State::Unknown,
            TriggerState::Processing => State::Processing,
            TriggerState::Running => State::Running,
            TriggerState::Exited => State::Exited,
        }
    }
}

impl From<State> for TriggerState {
    fn from(r: State) -> Self {
        match r {
            State::Unknown => TriggerState::UnknownState,
            State::Processing => TriggerState::Processing,
            State::Running => TriggerState::Running,
            State::Exited => TriggerState::Exited,
        }
    }
}

/// Triggers can be enabled and disabled.
#[derive(Debug, Display, EnumString, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum Status {
    /// Cannot determine status of Trigger, should never be in this status.
    Unknown,
    /// Installed and able to be used by pipelines.
    Enabled,
    /// Not available to be used by pipelines, either through lack of installation or
    /// being disabled by an admin.
    Disabled,
}

impl From<TriggerStatus> for Status {
    fn from(r: TriggerStatus) -> Self {
        match r {
            TriggerStatus::UnknownStatus => Status::Unknown,
            TriggerStatus::Enabled => Status::Enabled,
            TriggerStatus::Disabled => Status::Disabled,
        }
    }
}

impl From<Status> for TriggerStatus {
    fn from(r: Status) -> Self {
        match r {
            Status::Unknown => TriggerStatus::UnknownStatus,
            Status::Enabled => TriggerStatus::Enabled,
            Status::Disabled => TriggerStatus::Disabled,
        }
    }
}

/// The in-memory representation of a trigger, because triggers are somewhat ephemeral, many of the items listed
/// here quickly go out of date and are not worth storing in the database.
/// Because of this we keep an in-memory representation and store only trigger registrations.
#[derive(Debug, Clone)]
pub struct Trigger {
    pub registration: Registration,
    /// URL is the network address used to communicate with the trigger by the main process.
    pub url: Option<String>,
    /// SchedulerID is an identifier used by the scheduler to point out which container this trigger is mapped to. Used
    /// when manipulating the container through the identifier.
    pub scheduler_id: Option<String>,
    pub started: u64,
    pub state: State,
    pub status: Status,
    pub documentation: Option<String>,
    /// Key is a trigger's authentication key used to validate requests from the Gofer main service.
    /// On every request the Gofer service passes this key so that it is impossible for other service to contact
    /// and manipulate triggers directly.
    pub key: Option<String>,
}

impl From<Trigger> for gofer_proto::Trigger {
    fn from(ns: Trigger) -> Self {
        gofer_proto::Trigger {
            image: ns.registration.image,
            name: ns.registration.name,
            url: ns.url.unwrap_or_default(),
            scheduler_id: ns.scheduler_id.unwrap_or_default(),
            started: ns.started,
            state: gofer_proto::trigger::TriggerState::from(ns.state) as i32,
            status: gofer_proto::trigger::TriggerStatus::from(ns.status) as i32,
            documentation: ns.documentation.unwrap_or_default(),
        }
    }
}

/// When installing a new trigger, we allow the trigger installer to pass a bunch of settings that
/// allow us to go get that trigger on future startups.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct Registration {
    pub name: String,
    pub image: String,
    pub user: Option<String>,
    pub pass: Option<String>,
    pub variables: HashMap<String, String>,
    pub created: u64,
    pub status: Status,
}

impl From<gofer_proto::InstallTriggerRequest> for Registration {
    fn from(v: gofer_proto::InstallTriggerRequest) -> Self {
        Self {
            name: v.name,
            image: v.image,
            user: {
                if v.user.is_empty() {
                    None
                } else {
                    Some(v.user)
                }
            },
            pass: {
                if v.pass.is_empty() {
                    None
                } else {
                    Some(v.pass)
                }
            },
            variables: v.variables,
            created: super::epoch(),
            status: Status::Enabled,
        }
    }
}
