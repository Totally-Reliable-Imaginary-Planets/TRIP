use common_game::components::planet::{Planet, PlanetType};
use common_game::components::resource::BasicResourceType;
use common_game::protocols::orchestrator_planet::{OrchestratorToPlanet, PlanetToOrchestrator};
use common_game::protocols::planet_explorer::ExplorerToPlanet;
use log::{debug, error, info};

mod ai;

use crate::ai::AI;

/// Constructs and returns a fully initialized [`Planet`] instance for our group.
///
/// This function is the public entry point used by other groups' orchestrators
/// to instantiate our planet.
///
/// # Behavior
///
/// - Creates a new [`AI`] instance for this planet type.
/// - Configures the planet with our group's predefined generation and combination rules.
/// - Initializes the internal [`Planet`] using [`Planet::new`] and returns it.
///
/// # Parameters
///
/// - `id`: The planet's unique identifier within the galaxy.
/// - `orch_to_planet`: Receiver for orchestrator-to-planet messages.
/// - `planet_to_orch`: Sender for planet-to-orchestrator messages.
/// - `expl_to_planet`: Receiver for explorer-to-planet messages.
///
/// # Returns
///
/// - `Ok(Planet)` on successful construction.
///
/// # Errors
///
/// - `Err(String)` if [`Planet::new`] fails due to invalid parameters.
///
///
/// # Examples
///
/// let planet = trip(id, orch_rx, planet_tx, expl_rx)?;
/// spawn_planet_thread(planet);
///
/// # See Also
/// - [`Planet::new`]
/// - [`AI`]
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

    info!("planet_id={id} initialized");
    Ok(planet)
}

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
