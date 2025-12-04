use common_game::components::planet::{Planet, PlanetType};
use common_game::components::resource::BasicResourceType;
use common_game::protocols::messages::{
    ExplorerToPlanet, OrchestratorToPlanet, PlanetToExplorer, PlanetToOrchestrator,
};
use std::sync::mpsc;

mod ai;

use crate::ai::AI;

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
        orch_to_planet: mpsc::Receiver<OrchestratorToPlanet>,
        planet_to_orch: mpsc::Sender<PlanetToOrchestrator>,
        expl_to_planet: mpsc::Receiver<ExplorerToPlanet>,
        planet_to_expl: mpsc::Sender<PlanetToExplorer>,
    ) -> Result<Self, String> {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use common_game::components::asteroid::Asteroid;
    use common_game::components::sunray::Sunray;
    use common_game::protocols::messages::OrchestratorToPlanet;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_planet_creation() {
        let (_orch_tx, orch_rx) = mpsc::channel();
        let (planet_tx, _planet_rx) = mpsc::channel();
        let (_expl_tx, expl_rx) = mpsc::channel();
        let (planet_tx2, _planet_rx2) = mpsc::channel();

        let trip = Trip::new(0, orch_rx, planet_tx, expl_rx, planet_tx2);
        assert!(trip.is_ok());
    }

    #[test]
    fn test_planet_new_with_closed_channels() {
        let (orch_tx, orch_rx) = mpsc::channel();
        let (planet_tx, _planet_rx) = mpsc::channel();
        let (expl_tx, expl_rx) = mpsc::channel();
        let (planet_tx2, _planet_rx2) = mpsc::channel();

        // Close channels by dropping senders
        drop(orch_tx);
        drop(expl_tx);

        let result = Trip::new(1, orch_rx, planet_tx, expl_rx, planet_tx2);
        assert!(result.is_err());
    }

    #[test]
    fn test_planet_run() {
        let (orch_tx, orch_rx) = mpsc::channel();
        let (planet_tx, _planet_rx) = mpsc::channel();
        let (expl_tx, expl_rx) = mpsc::channel();
        let (planet_tx2, _planet_rx2) = mpsc::channel();

        let mut trip = Trip::new(0, orch_rx, planet_tx, expl_rx, planet_tx2).unwrap();

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
    fn test_planet_sunray_ack() {
        let (orch_tx, orch_rx) = mpsc::channel();
        let (planet_tx, planet_rx) = mpsc::channel();
        let (expl_tx, expl_rx) = mpsc::channel();
        let (planet_tx2, _planet_rx2) = mpsc::channel();

        let mut trip = Trip::new(0, orch_rx, planet_tx, expl_rx, planet_tx2).unwrap();

        let handle = thread::spawn(move || trip.run());

        orch_tx
            .send(OrchestratorToPlanet::StartPlanetAI)
            .expect("Failed to send start message");

        orch_tx
            .send(OrchestratorToPlanet::Sunray(Sunray::default()))
            .expect("Failed to send sunray message");

        match planet_rx
            .recv_timeout(Duration::from_millis(100))
            .expect("No message received")
        {
            PlanetToOrchestrator::SunrayAck { planet_id: 0 } => {}
            _other => panic!("Wrong response received"),
        }

        orch_tx
            .send(OrchestratorToPlanet::StopPlanetAI)
            .expect("Failed to send start message");

        drop(orch_tx);
        drop(expl_tx);

        let result = handle.join();
        assert!(result.is_ok());
    }

    #[test]
    fn test_planet_asteroid_ack() {
        let (orch_tx, orch_rx) = mpsc::channel();
        let (planet_tx, planet_rx) = mpsc::channel();
        let (expl_tx, expl_rx) = mpsc::channel();
        let (planet_tx2, _planet_rx2) = mpsc::channel();

        let mut trip = Trip::new(0, orch_rx, planet_tx, expl_rx, planet_tx2).unwrap();

        let handle = thread::spawn(move || trip.run());

        orch_tx
            .send(OrchestratorToPlanet::StartPlanetAI)
            .expect("Failed to send start message");

        orch_tx
            .send(OrchestratorToPlanet::Asteroid(Asteroid::default()))
            .expect("Failed to send asteroid message");

        match planet_rx
            .recv_timeout(Duration::from_millis(100))
            .expect("No message received")
        {
            PlanetToOrchestrator::AsteroidAck {
                rocket: None,
                planet_id: 0,
            } => {}
            _other => panic!("Wrong response received"),
        }

        orch_tx
            .send(OrchestratorToPlanet::StopPlanetAI)
            .expect("Failed to send start message");

        drop(orch_tx);
        drop(expl_tx);

        let result = handle.join();
        assert!(result.is_ok());
    }

    #[test]
    fn test_planet_survive_asteroid() {
        let (orch_tx, orch_rx) = mpsc::channel();
        let (planet_tx, planet_rx) = mpsc::channel();
        let (expl_tx, expl_rx) = mpsc::channel();
        let (planet_tx2, _planet_rx2) = mpsc::channel();

        let mut trip = Trip::new(0, orch_rx, planet_tx, expl_rx, planet_tx2).unwrap();

        let handle = thread::spawn(move || trip.run());

        orch_tx
            .send(OrchestratorToPlanet::StartPlanetAI)
            .expect("Failed to send start message");

        orch_tx
            .send(OrchestratorToPlanet::Sunray(Sunray::default()))
            .expect("Failed to send sunray message");

        match planet_rx
            .recv_timeout(Duration::from_millis(100))
            .expect("No message received")
        {
            PlanetToOrchestrator::SunrayAck { planet_id: 0 } => {}
            _other => panic!("Wrong response received"),
        }

        orch_tx
            .send(OrchestratorToPlanet::Asteroid(Asteroid::default()))
            .expect("Failed to send asteroid message");

        match planet_rx
            .recv_timeout(Duration::from_millis(100))
            .expect("No message received")
        {
            PlanetToOrchestrator::AsteroidAck {
                rocket: Some(_),
                planet_id: 0,
            } => {}
            _other => panic!("Wrong response received"),
        }

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

        let mut trip = Trip::new(1, orch_rx, planet_tx, expl_rx, planet_tx2).unwrap();

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
