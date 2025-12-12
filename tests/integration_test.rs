use common_game::components::asteroid::Asteroid;
use common_game::components::sunray::Sunray;
use common_game::protocols::messages::ExplorerToPlanet;
use common_game::protocols::messages::OrchestratorToPlanet;
use common_game::protocols::messages::OrchestratorToPlanet::IncomingExplorerRequest;
use common_game::protocols::messages::PlanetToExplorer;
use common_game::protocols::messages::PlanetToOrchestrator;
use std::thread;
use trip::trip;

use std::sync::Once;

static INIT: Once = Once::new();

fn setup_logger() {
    INIT.call_once(|| {
        env_logger::builder().is_test(true).init();
    });
}

mod common;

#[test]
fn test_planet_run() {
    setup_logger();
    let (orch_tx, orch_rx) = crossbeam_channel::unbounded();
    let (planet_tx, _planet_rx) = crossbeam_channel::unbounded();
    let (expl_tx, expl_rx) = crossbeam_channel::unbounded();

    let mut trip = trip(0, orch_rx, planet_tx, expl_rx).unwrap();

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
    setup_logger();
    let (orch_tx, orch_rx) = crossbeam_channel::unbounded();
    let (planet_tx, _planet_rx) = crossbeam_channel::unbounded();
    let (_expl_tx, expl_rx) = crossbeam_channel::unbounded();

    let mut trip = trip(1, orch_rx, planet_tx, expl_rx).unwrap();

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

#[test]
fn test_planet_supported_resource_resp() {
    setup_logger();
    let harness = common::TestHarness::setup();
    harness.start();
    let (expl_tx, expl_rx) = crossbeam_channel::unbounded();

    harness
        .orch_tx
        .send(IncomingExplorerRequest {
            explorer_id: 0,
            new_mpsc_sender: expl_tx,
        })
        .expect("Failed to send sunray message");

    harness
        .expl_tx
        .send(ExplorerToPlanet::SupportedResourceRequest { explorer_id: 0 })
        .expect("Failed to send asteroid message");

    match expl_rx.recv().expect("No message received") {
        PlanetToExplorer::SupportedResourceResponse { .. } => {}
        _other => panic!("Wrong response received"),
    }

    let result = harness.stop_and_join();
    assert!(result.is_ok());
}

#[test]
fn test_planet_supported_combination_resp() {
    setup_logger();
    let harness = common::TestHarness::setup();
    harness.start();
    let (expl_tx, expl_rx) = crossbeam_channel::unbounded();

    harness
        .orch_tx
        .send(IncomingExplorerRequest {
            explorer_id: 0,
            new_mpsc_sender: expl_tx,
        })
        .expect("Failed to send sunray message");

    harness
        .expl_tx
        .send(ExplorerToPlanet::SupportedCombinationRequest { explorer_id: 0 })
        .expect("Failed to send asteroid message");

    match expl_rx.recv().expect("No message received") {
        PlanetToExplorer::SupportedCombinationResponse { .. } => {}
        _other => panic!("Wrong response received"),
    }

    let result = harness.stop_and_join();
    assert!(result.is_ok());
}

#[test]
fn test_planet_available_eng_cell_resp() {
    setup_logger();
    let harness = common::TestHarness::setup();
    harness.start();
    let (expl_tx, expl_rx) = crossbeam_channel::unbounded();

    harness
        .orch_tx
        .send(IncomingExplorerRequest {
            explorer_id: 0,
            new_mpsc_sender: expl_tx,
        })
        .expect("Failed to send sunray message");

    harness
        .expl_tx
        .send(ExplorerToPlanet::AvailableEnergyCellRequest { explorer_id: 0 })
        .expect("Failed to send asteroid message");

    match expl_rx.recv().expect("No message received") {
        PlanetToExplorer::AvailableEnergyCellResponse { available_cells: 0 } => {}
        _other => panic!("Wrong response received"),
    }

    let result = harness.stop_and_join();
    assert!(result.is_ok());
}

#[test]
fn test_planet_sunray_ack() {
    setup_logger();
    let harness = common::TestHarness::setup();
    harness.start();

    harness
        .orch_tx
        .send(OrchestratorToPlanet::Sunray(Sunray::default()))
        .expect("Failed to send sunray message");

    let result = harness.recv_pto_with_timeout();
    match result {
        PlanetToOrchestrator::SunrayAck { planet_id: 0 } => {}
        _other => panic!("Wrong response received"),
    }
    harness
        .orch_tx
        .send(OrchestratorToPlanet::InternalStateRequest)
        .expect(
            format!(
                "Failed to send {:?} message",
                OrchestratorToPlanet::InternalStateRequest
            )
            .as_str(),
        );

    let result = harness.recv_pto_with_timeout();
    match result {
        PlanetToOrchestrator::InternalStateResponse {
            planet_state,
            planet_id: 0,
        } => {
            assert_eq!(
                planet_state.charged_cells_count, 0,
                "Charged cell must be 0"
            );
            assert!(planet_state.has_rocket, "Planet must have rocket");
        }
        _other => panic!("Wrong response received"),
    }

    let result = harness.stop_and_join();
    assert!(result.is_ok());
}

#[test]
fn test_planet_multiple_sunray_ack() {
    setup_logger();
    let harness = common::TestHarness::setup();
    harness.start();

    for _ in 0..20 {
        harness
            .orch_tx
            .send(OrchestratorToPlanet::Sunray(Sunray::default()))
            .expect("Failed to send sunray message");

        let result = harness.recv_pto_with_timeout();

        match result {
            PlanetToOrchestrator::SunrayAck { planet_id: 0 } => {}
            _other => panic!("Wrong response received"),
        }
    }
    harness
        .orch_tx
        .send(OrchestratorToPlanet::InternalStateRequest)
        .expect(
            format!(
                "Failed to send {:?} message",
                OrchestratorToPlanet::InternalStateRequest
            )
            .as_str(),
        );
    let result = harness.recv_pto_with_timeout();
    match result {
        PlanetToOrchestrator::InternalStateResponse {
            planet_state,
            planet_id: 0,
        } => {
            assert_eq!(
                planet_state.charged_cells_count, 5,
                "Charged cell must be 5"
            );
            assert!(planet_state.has_rocket, "Planet must have rocket");
        }
        _other => panic!("Wrong response received"),
    }

    let result = harness.stop_and_join();
    assert!(result.is_ok());
}

#[test]
fn test_planet_asteroid_ack() {
    setup_logger();
    let harness = common::TestHarness::setup();
    harness.start();

    harness
        .orch_tx
        .send(OrchestratorToPlanet::Asteroid(Asteroid::default()))
        .expect("Failed to send asteroid message");

    match harness.recv_pto_with_timeout() {
        PlanetToOrchestrator::AsteroidAck {
            rocket: None,
            planet_id: 0,
        } => {}
        _other => panic!("Wrong response received"),
    }

    let result = harness.join();
    assert!(result.is_ok());
}

#[test]
fn test_planet_survive_asteroid() {
    setup_logger();
    let harness = common::TestHarness::setup();
    harness.start();

    harness
        .orch_tx
        .send(OrchestratorToPlanet::Sunray(Sunray::default()))
        .expect("Failed to send sunray message");

    match harness.recv_pto_with_timeout() {
        PlanetToOrchestrator::SunrayAck { planet_id: 0 } => {}
        _other => panic!("Wrong response received"),
    }

    harness
        .orch_tx
        .send(OrchestratorToPlanet::Asteroid(Asteroid::default()))
        .expect("Failed to send asteroid message");

    match harness.recv_pto_with_timeout() {
        PlanetToOrchestrator::AsteroidAck {
            rocket: Some(_),
            planet_id: 0,
        } => {}
        _other => panic!("Wrong response received"),
    }

    let result = harness.stop_and_join();
    assert!(result.is_ok());
}

#[test]
fn test_planet_internal_state_resp() {
    setup_logger();
    let harness = common::TestHarness::setup();
    harness.start();

    harness
        .orch_tx
        .send(OrchestratorToPlanet::InternalStateRequest)
        .expect("Failed to send asteroid message");

    match harness.recv_pto_with_timeout() {
        PlanetToOrchestrator::InternalStateResponse { planet_id: 0, .. } => {}
        _other => panic!("Wrong response received"),
    }

    let result = harness.stop_and_join();
    assert!(result.is_ok());
}

#[test]
fn test_planet_incoming_expl_resp() {
    setup_logger();
    let harness = common::TestHarness::setup();
    harness.start();
    let (expl_tx, _expl_rx) = crossbeam_channel::unbounded();

    harness
        .orch_tx
        .send(OrchestratorToPlanet::IncomingExplorerRequest {
            explorer_id: 0,
            new_mpsc_sender: expl_tx,
        })
        .expect("Failed to send asteroid message");

    match harness.recv_pto_with_timeout() {
        PlanetToOrchestrator::IncomingExplorerResponse { planet_id: 0, .. } => {}
        _other => panic!("Wrong response received"),
    }

    let result = harness.stop_and_join();
    assert!(result.is_ok());
}

#[test]
fn test_planet_outgoing_expl_resp() {
    setup_logger();
    let harness = common::TestHarness::setup();
    harness.start();

    harness
        .orch_tx
        .send(OrchestratorToPlanet::OutgoingExplorerRequest { explorer_id: 0 })
        .expect("Failed to send asteroid message");

    match harness.recv_pto_with_timeout() {
        PlanetToOrchestrator::OutgoingExplorerResponse {
            planet_id: 0,
            res: Ok(()),
        } => {}
        _other => panic!("Wrong response received"),
    }

    let result = harness.stop_and_join();
    assert!(result.is_ok());
}
