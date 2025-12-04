use common_game::components::planet::{PlanetAI, PlanetState};
use common_game::components::resource::{Combinator, Generator};
use common_game::components::rocket::Rocket;
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
    fn handle_orchestrator_msg(
        &mut self,
        _: &mut PlanetState,
        _: &Generator,
        _: &Combinator,
        _: OrchestratorToPlanet,
    ) -> Option<PlanetToOrchestrator> {
        None
    }

    /// Handles a message from an explorer.
    fn handle_explorer_msg(
        &mut self,
        state: &mut PlanetState,
        _: &Generator,
        _: &Combinator,
        msg: ExplorerToPlanet,
    ) -> Option<PlanetToExplorer> {
        if self.is_stopped {
            return None;
        }
        match msg {
            ExplorerToPlanet::SupportedResourceRequest { .. }
            | ExplorerToPlanet::SupportedCombinationRequest { .. }
            | ExplorerToPlanet::GenerateResourceRequest { .. }
            | ExplorerToPlanet::CombineResourceRequest { .. } => todo!(),
            ExplorerToPlanet::AvailableEnergyCellRequest { .. } => {
                let tmp = state.cells_iter().filter(|&cell| cell.is_charged()).count();
                let count = tmp.try_into().unwrap_or_default();
                Some(PlanetToExplorer::AvailableEnergyCellResponse {
                    available_cells: count,
                })
            }
        }
    }

    /// Handles an incoming asteroid event.
    fn handle_asteroid(
        &mut self,
        _: &mut PlanetState,
        _: &Generator,
        _: &Combinator,
    ) -> Option<Rocket> {
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
