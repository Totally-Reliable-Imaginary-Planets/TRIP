use common_game::components::planet::{Planet, PlanetType};
use common_game::components::resource::BasicResourceType;
use common_game::protocols::messages::{
    ExplorerToPlanet, OrchestratorToPlanet, PlanetToOrchestrator,
};
use log::{debug, error, info};

mod ai;

use crate::ai::AI;

/// Creates a new Trip instance with the given parameters and initialized planet.
///
/// Attempts to receive initial messages from both the orchestrator and explorer channels
/// to verify connectivity. Initializes the internal `Planet` with the provided ID,
/// AI, resource rules, and communication channels.
///
/// # Errors
///
/// Returns an error if either the `orch_to_planet` or `expl_to_planet` channel is disconnected,
/// indicating that the corresponding sender has been dropped and communication cannot be established.
/// Specific error messages indicate which channel failed.
pub fn trip(
    id: u32,
    orch_to_planet: crossbeam_channel::Receiver<OrchestratorToPlanet>,
    planet_to_orch: crossbeam_channel::Sender<PlanetToOrchestrator>,
    expl_to_planet: crossbeam_channel::Receiver<ExplorerToPlanet>,
) -> Result<Planet, String> {
    match orch_to_planet.try_recv() {
        Err(crossbeam_channel::TryRecvError::Disconnected) => {
            error!("OrchestratorToPlanet channel is closed for planet {id}");
            return Err("OrchestratorToPlanet Channel is closed".to_string());
        }
        _ => debug!("ExplorerToPlanet channel open for planet {id}"),
    }
    match expl_to_planet.try_recv() {
        Err(crossbeam_channel::TryRecvError::Disconnected) => {
            return Err("ExplorerToPlanet channel is closed".to_string());
        }
        _ => debug!("ExplorerToPlanet channel open for planet {id}"),
    }
    let planet = Planet::new(
        id,
        PlanetType::A,
        Box::new(AI::new()),
        // gen rule
        vec![BasicResourceType::Oxygen],
        vec![],
        (orch_to_planet, planet_to_orch),
        expl_to_planet,
    )?;

    info!("Planet {id} initialized successfully");
    Ok(planet)
}
/*
/// The wrapper for the planet.
///
/// Holds a single `Planet` instance and manages its behavior through associated methods.
/// Used to encapsulate planet-specific logic and communication channels.
pub struct Trip {
    planet: Planet,
}

impl Trip {
    /// Creates a new Trip instance with the given parameters and initialized planet.
    ///
    /// Attempts to receive initial messages from both the orchestrator and explorer channels
    /// to verify connectivity. Initializes the internal `Planet` with the provided ID,
    /// AI, resource rules, and communication channels.
    ///
    /// # Errors
    ///
    /// Returns an error if either the `orch_to_planet` or `expl_to_planet` channel is disconnected,
    /// indicating that the corresponding sender has been dropped and communication cannot be established.
    /// Specific error messages indicate which channel failed.
    pub fn new(
        id: u32,
        orch_to_planet: crossbeam_channel::Receiver<OrchestratorToPlanet>,
        planet_to_orch: crossbeam_channel::Sender<PlanetToOrchestrator>,
        expl_to_planet: crossbeam_channel::Receiver<ExplorerToPlanet>,
        planet_to_expl: crossbeam_channel::Sender<PlanetToExplorer>,
    ) -> Result<Self, String> {
        match orch_to_planet.try_recv() {
            Err(crossbeam_channel::TryRecvError::Disconnected) => {
                return Err("OrchestratorToPlanet Channel is closed".to_string());
            }
            Err(crossbeam_channel::TryRecvError::Empty) => {
                println!("OrchestratorToPlanet channel is open but empty");
            }
            Ok(_) => println!("OrchestratorToPlanet channel open"),
        }
        match expl_to_planet.try_recv() {
            Err(crossbeam_channel::TryRecvError::Disconnected) => {
                return Err("ExplorerToPlanet channel is closed".to_string());
            }
            Err(crossbeam_channel::TryRecvError::Empty) => {
                println!("ExplorerToPlanet channel is open but empty");
            }
            Ok(_) => println!("ExplorerToPlanet channel open"),
        }
        let planet = Planet::new(
            id,
            PlanetType::A,
            Box::new(AI::new()),
            // gen rule
            vec![BasicResourceType::Oxygen],
            vec![],
            (orch_to_planet, planet_to_orch),
            (expl_to_planet, planet_to_expl),
        )?;
        Ok(Self { planet })
    }

    /// Runs the planet's operations and returns an error if something goes wrong.
    ///
    /// # Errors
    ///
    /// Returns an error if the planet fails to run due to internal malfunctions or external disruptions.
    pub fn run(&mut self) -> Result<(), String> {
        self.planet.run()
    }
}*/

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Once;

    static INIT: Once = Once::new();

    fn setup_logger() {
        INIT.call_once(|| {
            env_logger::builder().is_test(true).init();
        });
    }

    #[test]
    fn test_planet_creation() {
        setup_logger();
        let (_orch_tx, orch_rx) = crossbeam_channel::unbounded();
        let (planet_tx, _planet_rx) = crossbeam_channel::unbounded();
        let (_expl_tx, expl_rx) = crossbeam_channel::unbounded();

        let trip = trip(0, orch_rx, planet_tx, expl_rx);
        assert!(trip.is_ok());
    }

    #[test]
    fn test_planet_new_with_closed_channels() {
        setup_logger();
        let (orch_tx, orch_rx) = crossbeam_channel::unbounded();
        let (planet_tx, _planet_rx) = crossbeam_channel::unbounded();
        let (expl_tx, expl_rx) = crossbeam_channel::unbounded();

        // Close channels by dropping senders
        drop(orch_tx);
        drop(expl_tx);

        let result = trip(1, orch_rx, planet_tx, expl_rx);
        assert!(result.is_err());
    }
}
