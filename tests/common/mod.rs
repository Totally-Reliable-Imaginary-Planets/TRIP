use common_game::protocols::messages::ExplorerToPlanet;
use common_game::protocols::messages::OrchestratorToPlanet;
use common_game::protocols::messages::PlanetToOrchestrator;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use trip::trip;

// Helper struct to hold test resources
pub struct TestHarness {
    pub orch_tx: mpsc::Sender<OrchestratorToPlanet>,
    pub planet_rx: mpsc::Receiver<PlanetToOrchestrator>,
    pub expl_tx: mpsc::Sender<ExplorerToPlanet>,
    pub handle: thread::JoinHandle<Result<(), String>>,
}

impl TestHarness {
    pub fn setup() -> Self {
        let (orch_tx, orch_rx) = mpsc::channel();
        let (planet_tx, planet_rx) = mpsc::channel();
        let (expl_tx, expl_rx) = mpsc::channel();

        let mut trip = trip(0, orch_rx, planet_tx, expl_rx).unwrap();

        let handle = thread::spawn(move || trip.run());

        Self {
            orch_tx,
            planet_rx,
            expl_tx,
            handle,
        }
    }

    pub fn start(&self) {
        self.orch_tx
            .send(OrchestratorToPlanet::StartPlanetAI)
            .expect("Failed to send StartPlanetAI");
    }

    pub fn stop_and_join(self) -> thread::Result<Result<(), String>> {
        self.orch_tx
            .send(OrchestratorToPlanet::StopPlanetAI)
            .expect("Failed to send StopPlanetAI");
        drop(self.orch_tx);
        drop(self.expl_tx);
        self.handle.join()
    }

    pub fn join(self) -> thread::Result<Result<(), String>> {
        drop(self.orch_tx);
        drop(self.expl_tx);
        self.handle.join()
    }

    pub fn recv_pto_with_timeout(&self) -> PlanetToOrchestrator {
        self.planet_rx
            .recv_timeout(Duration::from_millis(100))
            .expect("No message received")
    }
}
