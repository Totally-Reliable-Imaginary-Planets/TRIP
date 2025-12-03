use common_game::components::planet::{Planet, PlanetAI, PlanetState, PlanetType};
use common_game::components::resource::{BasicResourceType, Combinator, Generator};
use common_game::components::rocket::Rocket;
use common_game::protocols::messages::{
    ExplorerToPlanet, OrchestratorToPlanet, PlanetToExplorer, PlanetToOrchestrator,
};
use std::sync::mpsc;

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
        match orch_to_planet.try_recv() {
            Err(mpsc::TryRecvError::Disconnected) => {
                return Err("OrchestratorToPlanet Channel is closed".to_string());
            }
            Err(mpsc::TryRecvError::Empty) => {
                println!("OrchestratorToPlanet channel is open but empty");
            }
            Ok(_) => println!("OrchestratorToPlanet channel open"),
        }
        match expl_to_planet.try_recv() {
            Err(mpsc::TryRecvError::Disconnected) => {
                return Err("ExplorerToPlanet channel is closed".to_string());
            }
            Err(mpsc::TryRecvError::Empty) => {
                println!("ExplorerToPlanet channel is open but empty");
            }
            Ok(_) => println!("ExplorerToPlanet channel open"),
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use common_game::protocols::messages::OrchestratorToPlanet;
    use std::sync::mpsc;
    use std::thread;

    #[test]
    fn test_trip_creation() {
        let (_orch_tx, orch_rx) = mpsc::channel();
        let (planet_tx, _planet_rx) = mpsc::channel();
        let (_expl_tx, expl_rx) = mpsc::channel();
        let (planet_tx2, _planet_rx2) = mpsc::channel();

        let trip = TRIP::new(0, orch_rx, planet_tx, expl_rx, planet_tx2);
        assert!(trip.is_ok());
    }

    #[test]
    fn test_trip_new_with_closed_channels() {
        let (orch_tx, orch_rx) = mpsc::channel();
        let (planet_tx, _planet_rx) = mpsc::channel();
        let (expl_tx, expl_rx) = mpsc::channel();
        let (planet_tx2, _planet_rx2) = mpsc::channel();

        // Close channels by dropping senders
        drop(orch_tx);
        drop(expl_tx);

        let result = TRIP::new(1, orch_rx, planet_tx, expl_rx, planet_tx2);
        assert!(result.is_err());
    }

    #[test]
    fn test_trip_run() {
        let (orch_tx, orch_rx) = mpsc::channel();
        let (planet_tx, _planet_rx) = mpsc::channel();
        let (expl_tx, expl_rx) = mpsc::channel();
        let (planet_tx2, _planet_rx2) = mpsc::channel();

        let mut trip = TRIP::new(0, orch_rx, planet_tx, expl_rx, planet_tx2).unwrap();

        let handle = thread::spawn(move || trip.run());

        orch_tx
            .send(OrchestratorToPlanet::StartPlanetAI)
            .expect("Failed to send start message");
        orch_tx
            .send(OrchestratorToPlanet::StopPlanetAI)
            .expect("Failed to send start message");

        drop(orch_tx);
        drop(expl_tx);

        let result = handle.join();
        assert!(result.is_ok());
    }

    #[test]
    fn test_concurrent_message_sending() {
        let (orch_tx, orch_rx) = mpsc::channel();
        let (planet_tx, _planet_rx) = mpsc::channel();
        let (_expl_tx, expl_rx) = mpsc::channel();
        let (planet_tx2, _planet_rx2) = mpsc::channel();

        let mut trip = TRIP::new(1, orch_rx, planet_tx, expl_rx, planet_tx2).unwrap();

        let handle = std::thread::spawn(move || {
            for _ in 0..100 {
                trip.run().ok();
            }
        });

        // Send many messages from multiple threads
        let tx1 = orch_tx.clone();
        let t1 = std::thread::spawn(move || {
            for _ in 0..50 {
                tx1.send(OrchestratorToPlanet::StartPlanetAI).unwrap();
            }
        });

        // Send many messages from multiple threads
        let tx2 = orch_tx.clone();
        let t2 = std::thread::spawn(move || {
            for _ in 0..50 {
                tx2.send(OrchestratorToPlanet::StopPlanetAI).unwrap();
            }
        });

        t1.join().unwrap();
        t2.join().unwrap();
        drop(orch_tx);

        let result = handle.join();
        assert!(result.is_ok());
    }
}
