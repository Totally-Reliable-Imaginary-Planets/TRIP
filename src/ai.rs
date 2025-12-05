use common_game::components::energy_cell::EnergyCell;
use common_game::components::planet::{PlanetAI, PlanetState};
use common_game::components::resource::BasicResourceType;
use common_game::components::resource::{Combinator, Generator};
use common_game::components::rocket::Rocket;
use common_game::protocols::messages::PlanetToOrchestrator::SunrayAck;
use common_game::protocols::messages::{
    ExplorerToPlanet, OrchestratorToPlanet, PlanetToExplorer, PlanetToOrchestrator,
};

/// The AI implementation for our planet
pub(crate) struct AI {
    is_stopped: bool,
}

impl AI {
    pub(crate) fn new() -> Self {
        Self { is_stopped: true }
    }
}

impl PlanetAI for AI {
    /// Called when the planet starts.
    fn start(&mut self, _: &PlanetState) {
        self.is_stopped = false;
    }

    /// Called when the planet stops.
    fn stop(&mut self, _: &PlanetState) {
        self.is_stopped = true;
    }

    /// Handles a message from the orchestrator.
    ///
    /// This method processes incoming messages from the orchestrator when the planet is active.
    /// If the planet is stopped (`self.is_stopped`), no messages are processed and `None` is returned immediately.
    ///
    /// # Behavior by Message Type
    ///
    /// - [`OrchestratorToPlanet::Sunray(s)`]:
    ///   - Finds the first uncharged cell and charges it with the sunray data.
    ///   - Attempts to build a rocket on that cell.
    ///   - Always returns a [`SunrayAck`] containing the planet ID.
    ///
    /// - [`OrchestratorToPlanet::IncomingExplorerRequest`], [`OrchestratorToPlanet::OutgoingExplorerRequest`],
    ///   [`OrchestratorToPlanet::InternalStateRequest`]:
    ///   - Marked with `todo!()` — these will panic in release and should be implemented.
    ///
    /// - [`OrchestratorToPlanet::Asteroid`], [`OrchestratorToPlanet::StartPlanetAI`], [`OrchestratorToPlanet::StopPlanetAI`]:
    ///   - Silently ignored (`None` returned).
    ///
    /// # Returns
    ///
    /// - `Some(PlanetToOrchestrator)`: A response is generated.
    /// - `None`: No response is sent, either because the planet is stopped or the message is ignored.
    ///
    /// # Logging
    ///
    /// # Panics
    ///
    /// Panics if:
    /// - An unimplemented message variant (`IncomingExplorerRequest`, etc.) is received.
    ///
    /// # See Also
    ///
    /// - [`PlanetState::build_rocket`]
    /// - [`SunrayAck`]
    fn handle_orchestrator_msg(
        &mut self,
        state: &mut PlanetState,
        _: &Generator,
        _: &Combinator,
        msg: OrchestratorToPlanet,
    ) -> Option<PlanetToOrchestrator> {
        if self.is_stopped {
            return None;
        }
        match msg {
            OrchestratorToPlanet::Sunray(s) => {
                if let Some(index) = state.cells_iter().position(|cell| !cell.is_charged()) {
                    let cell = state.cell_mut(index);
                    cell.charge(s);
                    match state.build_rocket(index) {
                        Ok(()) => println!("Rocket built successfully"),
                        Err(e) => println!("Rocekt Failed to be built: {e}"),
                    }
                }
                Some(SunrayAck {
                    planet_id: state.id(),
                })
            }
            OrchestratorToPlanet::InternalStateRequest => todo!(),
            OrchestratorToPlanet::OutgoingExplorerRequest { .. }
            | OrchestratorToPlanet::IncomingExplorerRequest { .. }
            | OrchestratorToPlanet::Asteroid(_)
            | OrchestratorToPlanet::StartPlanetAI
            | OrchestratorToPlanet::StopPlanetAI => None,
        }
    }

    /// Handles incoming messages from an `Explorer` agent and generates appropriate responses based on the planet's current state.
    ///
    /// This function processes various types of requests such as resource availability, combination support,
    /// generation, and energy cell status. If the planet is stopped (shut down or inactive), no responses are sent.
    ///
    /// # Parameters
    ///
    /// * `self`: Mutable reference to the planet's controller or handler, which includes runtime state like `is_stopped`.
    /// * `state`: Mutable reference to the current `PlanetState`, providing access to data like energy cells, resources, etc.
    /// * `generator`: reference for `Generator`.
    /// * `comb`: reference for `Combinator`.
    /// * `msg`: The incoming message from the explorer, wrapped in the `ExplorerToPlanet` enum.
    ///
    /// # Returns
    ///
    /// Returns an `Option<PlanetToExplorer>`:
    /// - `Some(response)` if a valid response can be generated.
    /// - `None` if the planet is stopped or the message type is unsupported/not yet implemented.
    ///
    /// # Message Handling
    ///
    /// Currently supports:
    /// - `AvailableEnergyCellRequest`: Responds with the count of charged energy cells.
    /// - `SupportedCombinationRequest`: Respond with the list of available comination recipes so
    ///   an empty hashset
    /// - `CombineResourceRequest`: Responde with the complex rescourc this planet can generate so
    ///   `None`
    /// - `SupportedResourceRequest`: Responds with the basic resource type hashset containing the
    ///   only supported resource `Oxygen`
    /// - `GenerateResourceRequest`: Responds only to request for the `Oxygen` resource althought
    ///   return `None`
    ///
    /// # Panics
    ///
    /// Panics if a non-implemented message variant is received.
    fn handle_explorer_msg(
        &mut self,
        state: &mut PlanetState,
        generator: &Generator,
        comb: &Combinator,
        msg: ExplorerToPlanet,
    ) -> Option<PlanetToExplorer> {
        if self.is_stopped {
            return None;
        }
        match msg {
            ExplorerToPlanet::SupportedResourceRequest { explorer_id: _ } => {
                Some(PlanetToExplorer::SupportedResourceResponse {
                    resource_list: generator.all_available_recipes(),
                })
            }
            ExplorerToPlanet::GenerateResourceRequest {
                explorer_id: _,
                resource: BasicResourceType::Oxygen,
            } => state
                .cells_iter()
                .position(EnergyCell::is_charged)
                .and_then(|index| generator.make_oxygen(state.cell_mut(index)).ok())
                .map(|r| {
                    println!("Resource generated");
                    PlanetToExplorer::GenerateResourceResponse {
                        resource: Some(common_game::components::resource::BasicResource::Oxygen(r)),
                    }
                }),
            ExplorerToPlanet::GenerateResourceRequest { .. } => None,
            ExplorerToPlanet::SupportedCombinationRequest { .. } => {
                Some(PlanetToExplorer::SupportedCombinationResponse {
                    combination_list: comb.all_available_recipes(),
                })
            }
            ExplorerToPlanet::CombineResourceRequest { .. } => {
                /*Some(PlanetToExplorer::CombineResourceResponse {
                    complex_response: None,
                })*/
                None
            }
            ExplorerToPlanet::AvailableEnergyCellRequest { .. } => {
                let tmp = state.cells_iter().filter(|&cell| cell.is_charged()).count();
                let count = tmp.try_into().unwrap_or_default();
                Some(PlanetToExplorer::AvailableEnergyCellResponse {
                    available_cells: count,
                })
            }
        }
    }

    /// Handles an incoming asteroid event by launching an existing rocket or building a new one.
    ///
    /// # Behavior
    ///
    /// 1. **Launch**: If a rocket is already built (`state.has_rocket()`), it is launched immediately
    ///    and returned.
    /// 2. **Build & Launch**: If no rocket exists, the method searches for the first charged energy cell
    ///    and attempts to build a rocket on it. If successful, the newly built rocket is launched and returned.
    /// 3. **Failure**: Returns `None` if no rocket was available and construction failed or no charged cell existed.
    ///
    /// # Returns
    ///
    /// - `Some(Rocket)`: A rocket was successfully launched (either pre-existing or newly built).
    /// - `None`: No rocket was launched (no rocket present and build failed or no charged cell).
    ///
    /// # Side Effects
    ///
    /// - Mutates `state`: may consume a rocket via `take_rocket()` and modify cells during construction.
    /// - Prints log messages on build success or failure (consider using `log` crate instead of `println!`).
    ///
    fn handle_asteroid(
        &mut self,
        state: &mut PlanetState,
        _: &Generator,
        _: &Combinator,
    ) -> Option<Rocket> {
        if state.has_rocket() {
            return state.take_rocket();
        }
        if let Some(index) = state.cells_iter().position(EnergyCell::is_charged) {
            match state.build_rocket(index) {
                Ok(()) => {
                    println!("Rocket built successfully");
                    return state.take_rocket();
                }
                Err(e) => println!("Rocekt Failed to be built: {e}"),
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    //use common_game::components::planet::PlanetState;
    //use common_game::components::resource::{Combinator, Generator};
    //use common_game::components::sunray::Sunray;
    //use common_game::protocols::messages::{ExplorerToPlanet, OrchestratorToPlanet};

    #[test]
    fn test_ai_initial_state() {
        let ai = AI::new();
        assert!(ai.is_stopped, "AI should start in stopped state");
    }

    // Waiting for PlanetState to implement Default trait
    /*#[test]
    fn test_start_sets_running() {
        let mut ai = AI::new();
        let state = PlanetState::default();
        ai.start(&state);
        assert!(!ai.is_stopped, "AI should be running after start()");
    }

    #[test]
    fn test_stop_sets_stopped() {
        let mut ai = AI::new();
        let state = PlanetState::default();

        ai.start(&state); // Start first
        assert!(!ai.is_stopped);

        ai.stop(&state);
        assert!(ai.is_stopped, "AI should be stopped after stop()");
    }

    #[test]
    fn test_handle_orchestrator_msg_returns_none() {
        let mut ai = AI::new();
        let state = &mut PlanetState::default();
        let generator = &Generator::default();
        let combinator = &Combinator::default();
        let msg = OrchestratorToPlanet::Sunray(Sunray::default()); // Adjust based on actual enum

        let result = ai.handle_orchestrator_msg(state, generator, combinator, msg);
        assert!(
            !result.is_some(),
            "Expected no response from orchestrator message handler"
        );
    }

    #[test]
    fn test_handle_explorer_msg_returns_none() {
        let mut ai = AI::new();
        let state = &mut PlanetState::default();
        let generator = &Generator::default();
        let combinator = &Combinator::default();
        let msg = ExplorerToPlanet::SupportedResourceRequest { explorer_id: 0 }; // Adjust based on actual enum

        let result = ai.handle_explorer_msg(state, generator, combinator, msg);
        assert!(
            !result.is_some(),
            "Expected no response from explorer message handler"
        );
    }

    #[test]
    fn test_handle_asteroid_returns_none() {
        let mut ai = AI::new();
        let state = &mut PlanetState::default();
        let generator = &Generator::default();
        let combinator = &Combinator::default();

        let result = ai.handle_asteroid(state, generator, combinator);
        assert!(
            !result.is_some(),
            "Expected no rocket launched on asteroid event"
        );
    }*/
}
