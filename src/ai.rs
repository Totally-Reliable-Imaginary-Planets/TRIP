use common_game::components::energy_cell::EnergyCell;
use common_game::components::planet::{PlanetAI, PlanetState};
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
    /// # Example
    ///
    /// ```no_run
    /// # let mut planet = Planet::default();
    /// # let mut state = PlanetState::new(planet.id());
    /// # let msg = OrchestratorToPlanet::Sunray(SunrayData::default());
    /// let response = planet.handle_orchestrator_msg(&mut state, &Generator, &Combinator, msg);
    /// if let Some(PlanetToOrchestrator::SunrayAck(ack)) = response {
    ///     println!("Acknowledged by planet: {}", ack.planet_id);
    /// }
    /// ```
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
            OrchestratorToPlanet::IncomingExplorerRequest { .. }
            | OrchestratorToPlanet::OutgoingExplorerRequest { .. }
            | OrchestratorToPlanet::InternalStateRequest => todo!(),
            OrchestratorToPlanet::Asteroid(_)
            | OrchestratorToPlanet::StartPlanetAI
            | OrchestratorToPlanet::StopPlanetAI => None,
        }
    }

    /// Handles a message from an explorer.
    fn handle_explorer_msg(
        &mut self,
        _: &mut PlanetState,
        _: &Generator,
        _: &Combinator,
        _: ExplorerToPlanet,
    ) -> Option<PlanetToExplorer> {
        None
    }

    /// Handles an incoming asteroid event.
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
