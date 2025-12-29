//! AI logic for our Planet implementation.
//!
//! This module defines the [`AI`] struct, which implements the [`PlanetAI`]
//! trait from `common_game`. The AI controls how our planet reacts to
//! orchestrator messages, explorer requests, and asteroid events.
//!
//! # Overview
//!
//! The AI manages three major responsibilities:
//!
//! 1. **Lifecycle control** via `start()` and `stop()`.
//!    - When stopped, the AI rejects all messages and produces no output.
//! 2. **Message handling**
//!    - [`handle_orchestrator_msg`] processes messages from the orchestrator,
//!      including sunrays, internal state requests, and others.
//!    - [`handle_explorer_msg`] processes queries and requests from explorers
//!      related to energy, basic resources, supported combinations, and complex
//!      combinations.
//! 3. **Asteroid response logic**
//!    - [`handle_asteroid`] launches an existing rocket or attempts to build
//!      and launch a new one.
//!
//! # AI Runtime Model
//!
//! The AI maintains an internal `running: bool` flag.
//! - When `running == false`, the planet is considered inactive and **all
//!   incoming messages are ignored**.
//! - The orchestrator controls this state via `StartPlanetAI` and
//!   `StopPlanetAI` messages.
//!
//! The planet never blocks inside the AI; blocking occurs only in the
//! outer planet loop that receives messages from channels.
//!
//! # Supported Features
//!
//! The AI supports:
//! - **Sunray absorption and energy cell charging**
//! - **Rocket construction via charged cells**
//! - **Internal state reporting**
//! - **Basic resource handling for Oxygen**
//! - **Fallback error reporting for unsupported combinations**
//! - **Asteroid-triggered rocket launching**
//!
//! # Unsupported Features (as of current version)
//!
//! The following message types are acknowledged but **not implemented** and
//! return `None` (or panic if explicitly marked with `todo!()` in the code):
//!
//! - Incoming and outgoing explorer routing requests
//! - Complex resource generation beyond the Oxygen path
//! - Planet kill event (currently ignored; real implementation should finalize
//!   the planet's lifecycle)
//!
//! # Thread Safety and Side Effects
//!
//! - The AI mutates [`PlanetState`] extensively (charging cells, building and
//!   launching rockets, creating resources).
//! - Logging is performed using the `log` crate.
//! - No global state is modified, and the struct is `Send` + `Sync` via its
//!   field structure.
//!
//! # Protocol Guarantees
//!
//! This implementation respects the project protocol by:
//! - Never reading from channels directly.
//! - Producing a response only when required.
//! - Logging all relevant state transitions.
//! - Maintaining deterministic behavior (no randomness here).
//!
//! # See Also
//!
//! - [`PlanetState`]
//! - [`Generator`]
//! - [`Combinator`]
//! - [`PlanetAI` trait](common_game::components::planet::PlanetAI)

use common_game::components::energy_cell::EnergyCell;
use common_game::components::planet::DummyPlanetState;
use common_game::components::planet::{PlanetAI, PlanetState};
use common_game::components::resource::ComplexResourceRequest;
use common_game::components::resource::{
    BasicResource, BasicResourceType, Combinator, ComplexResource, Generator, GenericResource,
};
use common_game::components::rocket::Rocket;
use common_game::components::sunray::Sunray;
use common_game::protocols::planet_explorer::{ExplorerToPlanet, PlanetToExplorer};
use log::{debug, error, info, warn};

/// AI implementation for our planet.
///
/// This AI governs message handling, lifecycle control, energy management,
/// rocket building, resource generation, and asteroid defense.
///
/// See the module-level documentation for full details.
pub(crate) struct AI {
    running: bool,
}

impl AI {
    /// Creates a new, inactive [`AI`] instance.
    ///
    /// The AI begins in the `running = false` state, meaning no incoming
    /// messages will be processed until [`start`](PlanetAI::start) is called.
    pub(crate) fn new() -> Self {
        Self { running: false }
    }

    /// Returns `true` if the AI is currently active, otherwise logs that the
    /// AI ignored a message due to being stopped and returns `false`.
    ///
    /// # Parameters
    /// - `planet_id`: The ID of the planet for contextual logging.
    ///
    /// # Returns
    /// `true` if `running == true`, `false` otherwise.
    ///
    /// # Side Effects
    /// - Writes a debug log message when inactive.
    fn is_running(&self, planet_id: u32) -> bool {
        if !self.running {
            debug!("planet_id={planet_id} msg_ignored: ai_stopped");
            return false;
        }
        true
    }

    /// Transforms a [`ComplexResourceRequest`] into a pair of [`GenericResource`]
    /// values suitable for error reporting or unsupported-combination responses.
    ///
    /// This function does not validate whether the combination is allowed on
    /// this planet; it only decomposes the request into its constituent parts.
    ///
    /// # Returns
    /// A pair `(left, right)` representing the two logical inputs to a
    /// combination request.
    fn get_generic_resources(msg: ComplexResourceRequest) -> (GenericResource, GenericResource) {
        match msg {
            ComplexResourceRequest::Water(h, o) => (
                GenericResource::BasicResources(BasicResource::Hydrogen(h)),
                GenericResource::BasicResources(BasicResource::Oxygen(o)),
            ),
            ComplexResourceRequest::Diamond(c1, c2) => (
                GenericResource::BasicResources(BasicResource::Carbon(c1)),
                GenericResource::BasicResources(BasicResource::Carbon(c2)),
            ),
            ComplexResourceRequest::Life(w, c) => (
                GenericResource::ComplexResources(ComplexResource::Water(w)),
                GenericResource::BasicResources(BasicResource::Carbon(c)),
            ),
            ComplexResourceRequest::Robot(s, l) => (
                GenericResource::BasicResources(BasicResource::Silicon(s)),
                GenericResource::ComplexResources(ComplexResource::Life(l)),
            ),
            ComplexResourceRequest::Dolphin(w, l) => (
                GenericResource::ComplexResources(ComplexResource::Water(w)),
                GenericResource::ComplexResources(ComplexResource::Life(l)),
            ),
            ComplexResourceRequest::AIPartner(r, d) => (
                GenericResource::ComplexResources(ComplexResource::Robot(r)),
                GenericResource::ComplexResources(ComplexResource::Diamond(d)),
            ),
        }
    }

    /// Handles a [`Sunray`] by charging the first uncharged energy cell and
    /// attempting to build a rocket on that cell.
    ///
    /// This method encapsulates the sunray-handling logic used by
    /// [`handle_orchestrator_msg`](PlanetAI::handle_orchestrator_msg).
    ///
    /// # Behavior
    /// - Charges the first available uncharged cell.
    /// - Attempts to build a rocket on that cell; logs success or failure.
    /// - Logs relevant diagnostic information.
    ///
    /// # Side Effects
    /// - Mutates the [`PlanetState`] (cell charge, rocket construction).
    /// - Emits debug, info, or error logs.
    fn handle_sunray(state: &mut PlanetState, s: Sunray) {
        debug!("planet_id={} incoming_sunray", state.id());
        if let Some(index) = state.cells_iter().position(|cell| !cell.is_charged()) {
            let cell = state.cell_mut(index);
            cell.charge(s);
            debug!("planet_id={} sunray: charging cell", state.id());
            match state.build_rocket(index) {
                Ok(()) => info!("planet_id={} rocket_built", state.id()),
                Err(e) => warn!("planet_id={} rocket_build_failed: {}", state.id(), e),
            }
        } else {
            warn!("planet_id={} sunray: no_uncharged_cells", state.id());
        }
        debug!("planet_id={} outgoing_sunray_ack", state.id());
    }
}

impl PlanetAI for AI {
    /// Activates the AI and enables message processing.
    ///
    /// Called by the planet runtime when initialization completes.
    /// After this call, incoming messages to the AI will be processed normally.
    ///
    /// # Side Effects
    /// - Sets `running = true`
    /// - Logs an informational `ai_started` message
    fn on_start(&mut self, state: &PlanetState, _: &Generator, _: &Combinator) {
        self.running = true;
        info!("planet_id={} ai_started", state.id());
    }

    /// Deactivates the AI and stops all message processing.
    ///
    /// All message handlers will return `None` until the AI is restarted.
    ///
    /// # Side Effects
    /// - Sets `running = false`
    /// - Logs an informational `ai_stopped` message
    fn on_stop(&mut self, state: &PlanetState, _: &Generator, _: &Combinator) {
        self.running = false;
        info!("planet_id={} ai_stopped", state.id());
    }

    /// Handles a sunray by delegating to the internal charging logic.
    ///
    /// # Behavior
    /// - Consumes the incoming sunray to charge the first available energy cell.
    /// - Attempts to build a rocket immediately after charging.
    /// - This is a wrapper around the static [`AI::handle_sunray`] method.
    fn handle_sunray(&mut self, state: &mut PlanetState, _: &Generator, _: &Combinator, s: Sunray) {
        if self.is_running(state.id()) {
            AI::handle_sunray(state, s);
        }
    }

    /// Provides a `DummyPlanetState` object representing the current planet state.
    ///
    /// # Behavior
    /// - Converts the current `PlanetState` into a `DummyPlanetState`.
    ///
    /// # Returns
    /// A `DummyPlanetState` representing the current state of the planet.
    fn handle_internal_state_req(
        &mut self,
        state: &mut PlanetState,
        _: &Generator,
        _: &Combinator,
    ) -> DummyPlanetState {
        state.to_dummy()
    }

    /// Handles messages from an explorer interacting with this planet.
    ///
    /// The AI responds to explorer queries about:
    /// - Supported basic resources
    /// - Supported combination rules
    /// - Energy availability
    /// - Requests to generate Oxygen
    ///
    /// Unsupported combinations or unsupported resource requests result in
    /// `None` or an appropriate error response.
    ///
    /// # Behavior
    ///
    /// - If the AI is stopped, returns `None`.
    /// - Basic resource generation is supported only for Oxygen.
    /// - Combination attempts always produce an `Err` payload indicating
    ///   unsupported functionality.
    ///
    /// # Returns
    /// - `Some(response)` if a valid response exists.
    /// - `None` if the AI is stopped or if the request cannot be fulfilled.    
    fn handle_explorer_msg(
        &mut self,
        state: &mut PlanetState,
        generator: &Generator,
        comb: &Combinator,
        msg: ExplorerToPlanet,
    ) -> Option<PlanetToExplorer> {
        if !self.is_running(state.id()) {
            return None;
        }
        match msg {
            ExplorerToPlanet::SupportedResourceRequest { explorer_id } => {
                debug!(
                    "planet_id={} explorer_id={} outgoing_supported_resource_response",
                    state.id(),
                    explorer_id
                );
                Some(PlanetToExplorer::SupportedResourceResponse {
                    resource_list: generator.all_available_recipes(),
                })
            }
            ExplorerToPlanet::GenerateResourceRequest {
                explorer_id,
                resource: BasicResourceType::Oxygen,
            } => state
                .cells_iter()
                .position(EnergyCell::is_charged)
                .and_then(|index| generator.make_oxygen(state.cell_mut(index)).ok())
                .map(|r| {
                    debug!(
                        "planet_id={} explorer_id={} generate_oxygen: success",
                        state.id(),
                        explorer_id
                    );
                    PlanetToExplorer::GenerateResourceResponse {
                        resource: Some(common_game::components::resource::BasicResource::Oxygen(r)),
                    }
                })
                .or_else(|| {
                    warn!(
                        "planet_id={} explorer_id={} generate_oxygen: failed",
                        state.id(),
                        explorer_id
                    );
                    None
                }),
            ExplorerToPlanet::GenerateResourceRequest { explorer_id, .. } => {
                debug!(
                    "planet_id={} explorer_id={} generate_resource: unsupported",
                    state.id(),
                    explorer_id
                );
                None
            }
            ExplorerToPlanet::SupportedCombinationRequest { explorer_id, .. } => {
                debug!(
                    "planet_id={} explorer_id={} outgoing_supported_combination_response",
                    state.id(),
                    explorer_id
                );
                Some(PlanetToExplorer::SupportedCombinationResponse {
                    combination_list: comb.all_available_recipes(),
                })
            }
            ExplorerToPlanet::CombineResourceRequest { explorer_id, msg } => {
                debug!(
                    "planet_id={} explorer_id={} incoming_combine_request: {:?}",
                    state.id(),
                    explorer_id,
                    msg
                );
                let (left, right) = AI::get_generic_resources(msg);
                debug!(
                    "planet_id={} explorer_id={} outgoing_combine_response=unsupported_combination",
                    state.id(),
                    explorer_id
                );
                Some(PlanetToExplorer::CombineResourceResponse {
                    complex_response: Err(("unsupported_combination".to_string(), left, right)),
                })
            }
            ExplorerToPlanet::AvailableEnergyCellRequest { explorer_id } => {
                let tmp = state.cells_iter().filter(|&cell| cell.is_charged()).count();
                let count = tmp.try_into().unwrap_or_default();
                debug!(
                    "planet_id={} explorer_id={} outgoing_energy_cell_count={}",
                    state.id(),
                    explorer_id,
                    count
                );
                Some(PlanetToExplorer::AvailableEnergyCellResponse {
                    available_cells: count,
                })
            }
        }
    }

    /// Handles an asteroid impact event.
    ///
    /// # Behavior
    ///
    /// - If a rocket already exists in the state, it is launched immediately.
    /// - Otherwise, the AI searches for the first charged energy cell and
    ///   attempts to build a rocket on it.
    /// - If rocket construction succeeds, the rocket is launched.
    /// - If construction fails or no charged cell exists, `None` is returned.
    ///
    /// # Side Effects
    /// - Mutates the planet state by consuming energy cells and creating rockets.
    /// - Logs informational or warning messages depending on outcome.
    ///
    /// # Returns
    /// `Some(Rocket)` if a rocket is launched, otherwise `None`.    
    fn handle_asteroid(
        &mut self,
        state: &mut PlanetState,
        _: &Generator,
        _: &Combinator,
    ) -> Option<Rocket> {
        if !self.is_running(state.id()) {
            return None;
        }
        if state.has_rocket() {
            info!(
                "planet_id={} asteroid_event: existing_rocket_launched",
                state.id()
            );
            return state.take_rocket();
        }
        if let Some(index) = state.cells_iter().position(EnergyCell::is_charged) {
            match state.build_rocket(index) {
                Ok(()) => {
                    info!(
                        "planet_id={} asteroid_event: rocket_built_and_launched",
                        state.id()
                    );
                    return state.take_rocket();
                }
                Err(e) => error!(
                    "planet_id={} asteroid_event: rocket_build_failed {}",
                    state.id(),
                    e
                ),
            }
        } else {
            warn!(
                "planet_id={} asteroid_event: no_charged_cells_available",
                state.id()
            );
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
        assert!(!ai.running, "AI should start in stopped state");
    }

    // Waiting for PlanetState to implement Default trait
    /*#[test]
    fn test_start_sets_running() {
        let mut ai = AI::new();
        let state = PlanetState::default();
        ai.start(&state);
        assert!(!ai.running, "AI should be running after start()");
    }

    #[test]
    fn test_stop_sets_stopped() {
        let mut ai = AI::new();
        let state = PlanetState::default();

        ai.start(&state); // Start first
        assert!(!ai.running);

        ai.stop(&state);
        assert!(ai.running, "AI should be stopped after stop()");
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
