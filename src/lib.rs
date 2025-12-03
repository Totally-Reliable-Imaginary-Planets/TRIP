use std::sync::mpsc;
use common_game::components::planet::{Planet, PlanetType, PlanetState, PlanetAI};
use common_game::protocols::messages::{PlanetToExplorer, OrchestratorToPlanet, ExplorerToPlanet, PlanetToOrchestrator};
use common_game::components::rocket::Rocket;
use common_game::components::resource::{Generator, Combinator, BasicResourceType};

pub struct AI;

impl PlanetAI for AI {
    fn start(&mut self, _: &PlanetState) {}
    fn stop(&mut self, _: &PlanetState) {}
    fn handle_orchestrator_msg(
        &mut self,
        _: &mut PlanetState,
        _: &Generator,
        _: &Combinator,
        _: OrchestratorToPlanet,
    ) -> Option<PlanetToOrchestrator> {
        None
    }
    fn handle_explorer_msg(
        &mut self,
        _: &mut PlanetState,
        _: &Generator,
        _: &Combinator,
        _: ExplorerToPlanet,
    ) -> Option<PlanetToExplorer> {
        None
    }
    fn handle_asteroid(
        &mut self,
        _: &mut PlanetState,
        _: &Generator,
        _: &Combinator,
    ) -> Option<Rocket> {
        None
    }
}

pub struct TRIP {
    planet: Planet,
}

impl TRIP {
    pub fn new(
        id: u32,
        orch_to_planet: mpsc::Receiver<OrchestratorToPlanet>,
        planet_to_orch: mpsc::Sender<PlanetToOrchestrator>,
        expl_to_planet: mpsc::Receiver<ExplorerToPlanet>,
        planet_to_expl: mpsc::Sender<PlanetToExplorer>,
    ) -> Result<TRIP, String> {
        let planet = Planet::new(
            id,
            PlanetType::A,
            Box::new(AI),
            // gen rule
            vec![BasicResourceType::Oxygen],
            vec![],
            (orch_to_planet, planet_to_orch),
            (expl_to_planet, planet_to_expl),
        )?;
        Ok(Self { planet })
    }

    pub fn run(&mut self) -> Result<(), String> {
        self.planet.run()
    }
}
