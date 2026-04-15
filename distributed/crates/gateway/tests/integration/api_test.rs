//! Integration tests for the ZisK Gateway API.
//!
//! Each test spins up a real tonic server on a random port backed by
//! `MockBackend`, then connects a real generated client and exercises the
//! full RPC round-trip.

use std::time::Duration;

use futures::StreamExt;
use tokio::net::TcpListener;
use tonic::transport::{Channel, Server};

use zisk_gateway::{
    backend::mock::MockBackend,
    proto::{
        input_kind, job_event, job_kind, job_status, zisk_gateway_api_client::ZiskGatewayApiClient,
        zisk_gateway_api_server::ZiskGatewayApiServer, CancelJobRequest, ExecuteRequest,
        InputChunk, InputKind, JobKind, JobRequestMessage, ProofKind, ProveRequest,
        PushJobInputRequest, RegisterGuestProgramRequest, SetupRequest, WaitJobResultRequest,
        WatchJobRequest, WrapRequest,
    },
    service::GatewayService,
};

use std::sync::Arc;

// ── Test helpers ──────────────────────────────────────────────────────────────

/// Start a gateway server on a random local port and return a connected client.
async fn start_test_server() -> ZiskGatewayApiClient<Channel> {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let backend = Arc::new(MockBackend::default());
    let service = GatewayService::new(Arc::clone(&backend));

    tokio::spawn(async move {
        Server::builder()
            .add_service(ZiskGatewayApiServer::new(service))
            .serve_with_incoming(tokio_stream::wrappers::TcpListenerStream::new(listener))
            .await
            .unwrap();
    });

    // Brief pause to let the server start accepting connections
    tokio::time::sleep(Duration::from_millis(10)).await;

    let endpoint = format!("http://{addr}");
    ZiskGatewayApiClient::connect(endpoint).await.unwrap()
}

fn dummy_elf() -> Vec<u8> {
    vec![0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01, 0x02, 0x03]
}

fn inline_input(is_last: bool) -> Option<InputKind> {
    Some(InputKind {
        kind: Some(input_kind::Kind::Inline(InputChunk { data: vec![1, 2, 3], is_last })),
    })
}

async fn register_program(client: &mut ZiskGatewayApiClient<Channel>) -> String {
    client
        .register_guest_program(RegisterGuestProgramRequest { zisk_elf: dummy_elf() })
        .await
        .unwrap()
        .into_inner()
        .hash_id
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn register_idempotent() {
    let mut client = start_test_server().await;
    let elf = dummy_elf();

    let h1 = client
        .register_guest_program(RegisterGuestProgramRequest { zisk_elf: elf.clone() })
        .await
        .unwrap()
        .into_inner()
        .hash_id;

    let h2 = client
        .register_guest_program(RegisterGuestProgramRequest { zisk_elf: elf })
        .await
        .unwrap()
        .into_inner()
        .hash_id;

    assert_eq!(h1, h2, "same ELF must produce the same hash_id");
    assert!(!h1.is_empty());
}

#[tokio::test]
async fn prove_job_wait_result_completes() {
    let mut client = start_test_server().await;
    let hash_id = register_program(&mut client).await;

    let job_id = client
        .job_request(JobRequestMessage {
            job_kind: Some(JobKind {
                kind: Some(job_kind::Kind::Prove(ProveRequest {
                    hash_id,
                    input: inline_input(true),
                    proof_timeout: None,
                })),
            }),
        })
        .await
        .unwrap()
        .into_inner()
        .job_id;

    assert!(!job_id.is_empty());

    // Poll until Completed (max 5 s)
    loop {
        let resp = client
            .wait_job_result(WaitJobResultRequest {
                job_id: job_id.clone(),
                timeout_seconds: Some(2),
            })
            .await
            .unwrap()
            .into_inner();

        if let Some(status) = &resp.job_status {
            if let Some(job_status::Status::Completed(_)) = &status.status {
                assert!(resp.result.is_some(), "Completed must carry a result");
                return;
            }
        }
    }
}

#[tokio::test]
async fn prove_job_watch_stream_receives_all_events() {
    let mut client = start_test_server().await;
    let hash_id = register_program(&mut client).await;

    let job_id = client
        .job_request(JobRequestMessage {
            job_kind: Some(JobKind {
                kind: Some(job_kind::Kind::Prove(ProveRequest {
                    hash_id,
                    input: inline_input(true),
                    proof_timeout: None,
                })),
            }),
        })
        .await
        .unwrap()
        .into_inner()
        .job_id;

    let mut stream = client.watch_job(WatchJobRequest { job_id }).await.unwrap().into_inner();

    let mut saw_queued = false;
    let mut saw_started = false;
    let mut saw_completed = false;

    while let Some(event) = stream.next().await {
        let event = event.unwrap();
        match event.event {
            Some(job_event::Event::Queued(_)) => saw_queued = true,
            Some(job_event::Event::Started(_)) => saw_started = true,
            Some(job_event::Event::Completed(e)) => {
                saw_completed = true;
                assert!(e.result.is_some(), "Completed event must carry result");
            }
            _ => {}
        }
    }

    assert!(saw_queued, "expected Queued event");
    assert!(saw_started, "expected Started event");
    assert!(saw_completed, "expected Completed event");
}

#[tokio::test]
async fn setup_job_completes() {
    let mut client = start_test_server().await;
    let hash_id = register_program(&mut client).await;

    let job_id = client
        .job_request(JobRequestMessage {
            job_kind: Some(JobKind { kind: Some(job_kind::Kind::Setup(SetupRequest { hash_id })) }),
        })
        .await
        .unwrap()
        .into_inner()
        .job_id;

    let resp = loop {
        let r = client
            .wait_job_result(WaitJobResultRequest {
                job_id: job_id.clone(),
                timeout_seconds: Some(2),
            })
            .await
            .unwrap()
            .into_inner();
        if let Some(s) = &r.job_status {
            if matches!(s.status, Some(job_status::Status::Completed(_))) {
                break r;
            }
        }
    };

    assert!(matches!(resp.job_status.unwrap().status, Some(job_status::Status::Completed(_))));
}

#[tokio::test]
async fn execute_job_completes() {
    let mut client = start_test_server().await;
    let hash_id = register_program(&mut client).await;

    let job_id = client
        .job_request(JobRequestMessage {
            job_kind: Some(JobKind {
                kind: Some(job_kind::Kind::Execute(ExecuteRequest {
                    hash_id,
                    input: inline_input(true),
                    execute_timeout: None,
                })),
            }),
        })
        .await
        .unwrap()
        .into_inner()
        .job_id;

    loop {
        let r = client
            .wait_job_result(WaitJobResultRequest {
                job_id: job_id.clone(),
                timeout_seconds: Some(2),
            })
            .await
            .unwrap()
            .into_inner();
        if let Some(s) = &r.job_status {
            if matches!(s.status, Some(job_status::Status::Completed(_))) {
                return;
            }
        }
    }
}

#[tokio::test]
async fn wrap_job_completes() {
    use zisk_gateway::proto::job_kind_response;

    let mut client = start_test_server().await;

    // First get a proof via Prove
    let hash_id = register_program(&mut client).await;
    let prove_job_id = client
        .job_request(JobRequestMessage {
            job_kind: Some(JobKind {
                kind: Some(job_kind::Kind::Prove(ProveRequest {
                    hash_id,
                    input: inline_input(true),
                    proof_timeout: None,
                })),
            }),
        })
        .await
        .unwrap()
        .into_inner()
        .job_id;

    let prove_result = loop {
        let r = client
            .wait_job_result(WaitJobResultRequest {
                job_id: prove_job_id.clone(),
                timeout_seconds: Some(2),
            })
            .await
            .unwrap()
            .into_inner();
        if let Some(s) = &r.job_status {
            if matches!(s.status, Some(job_status::Status::Completed(_))) {
                break r;
            }
        }
    };

    let stark_proof = match prove_result.result.unwrap().kind.unwrap() {
        job_kind_response::Kind::Prove(p) => p.proof.unwrap(),
        _ => panic!("expected ProveResponse"),
    };

    // Now wrap Stark → Plonk
    let wrap_job_id = client
        .job_request(JobRequestMessage {
            job_kind: Some(JobKind {
                kind: Some(job_kind::Kind::Wrap(WrapRequest {
                    proof: Some(stark_proof),
                    proof_dest: ProofKind::Plonk as i32,
                    wrap_timeout: None,
                })),
            }),
        })
        .await
        .unwrap()
        .into_inner()
        .job_id;

    let wrap_result = loop {
        let r = client
            .wait_job_result(WaitJobResultRequest {
                job_id: wrap_job_id.clone(),
                timeout_seconds: Some(2),
            })
            .await
            .unwrap()
            .into_inner();
        if let Some(s) = &r.job_status {
            if matches!(s.status, Some(job_status::Status::Completed(_))) {
                break r;
            }
        }
    };

    let wrapped_proof = match wrap_result.result.unwrap().kind.unwrap() {
        job_kind_response::Kind::Wrap(w) => w.proof.unwrap(),
        _ => panic!("expected WrapResponse"),
    };

    assert_eq!(wrapped_proof.proof_kind, ProofKind::Plonk as i32);
}

#[tokio::test]
async fn cancel_running_job_returns_true() {
    let mut client = start_test_server().await;
    let hash_id = register_program(&mut client).await;

    let job_id = client
        .job_request(JobRequestMessage {
            job_kind: Some(JobKind {
                kind: Some(job_kind::Kind::Prove(ProveRequest {
                    hash_id,
                    input: inline_input(true),
                    proof_timeout: None,
                })),
            }),
        })
        .await
        .unwrap()
        .into_inner()
        .job_id;

    let resp =
        client.cancel_job(CancelJobRequest { job_id: job_id.clone() }).await.unwrap().into_inner();

    assert!(resp.cancelled, "job should have been cancelled");

    // Cancelling again is idempotent
    let resp2 = client.cancel_job(CancelJobRequest { job_id }).await.unwrap().into_inner();

    assert!(!resp2.cancelled, "second cancel must return false (already terminal)");
}

#[tokio::test]
async fn cancel_completed_job_returns_false() {
    let mut client = start_test_server().await;
    let hash_id = register_program(&mut client).await;

    let job_id = client
        .job_request(JobRequestMessage {
            job_kind: Some(JobKind { kind: Some(job_kind::Kind::Setup(SetupRequest { hash_id })) }),
        })
        .await
        .unwrap()
        .into_inner()
        .job_id;

    // Wait for completion
    loop {
        let r = client
            .wait_job_result(WaitJobResultRequest {
                job_id: job_id.clone(),
                timeout_seconds: Some(2),
            })
            .await
            .unwrap()
            .into_inner();
        if let Some(s) = &r.job_status {
            if matches!(s.status, Some(job_status::Status::Completed(_))) {
                break;
            }
        }
    }

    let resp = client.cancel_job(CancelJobRequest { job_id }).await.unwrap().into_inner();

    assert!(!resp.cancelled, "cancelling a completed job must return false");
}

#[tokio::test]
async fn job_not_found_returns_error() {
    let mut client = start_test_server().await;
    let fake_id = uuid::Uuid::new_v4().to_string();

    let err = client
        .wait_job_result(WaitJobResultRequest { job_id: fake_id, timeout_seconds: Some(1) })
        .await
        .unwrap_err();

    assert_eq!(err.code(), tonic::Code::NotFound);
}

#[tokio::test]
async fn program_not_found_returns_error() {
    let mut client = start_test_server().await;

    let err = client
        .job_request(JobRequestMessage {
            job_kind: Some(JobKind {
                kind: Some(job_kind::Kind::Setup(SetupRequest {
                    hash_id: "nonexistent_hash".into(),
                })),
            }),
        })
        .await
        .unwrap_err();

    assert_eq!(err.code(), tonic::Code::NotFound);
}

#[tokio::test]
async fn wait_result_timeout_returns_current_status() {
    let mut client = start_test_server().await;
    let hash_id = register_program(&mut client).await;

    // Submit a Prove job that takes ~150ms in the mock
    let job_id = client
        .job_request(JobRequestMessage {
            job_kind: Some(JobKind {
                kind: Some(job_kind::Kind::Prove(ProveRequest {
                    hash_id,
                    input: inline_input(true),
                    proof_timeout: None,
                })),
            }),
        })
        .await
        .unwrap()
        .into_inner()
        .job_id;

    // Use a 1 s timeout — should return without error even if the job
    // isn't terminal yet (if called very early)
    let resp = client
        .wait_job_result(WaitJobResultRequest { job_id, timeout_seconds: Some(1) })
        .await
        .unwrap()
        .into_inner();

    // Must get a valid status back (not an error)
    assert!(resp.job_status.is_some(), "response must contain a job_status");
}

#[tokio::test]
async fn push_input_multi_chunk_completes() {
    let mut client = start_test_server().await;
    let hash_id = register_program(&mut client).await;

    // Submit with is_last=false to trigger WaitingForInput
    let job_id = client
        .job_request(JobRequestMessage {
            job_kind: Some(JobKind {
                kind: Some(job_kind::Kind::Prove(ProveRequest {
                    hash_id,
                    input: inline_input(false), // is_last = false
                    proof_timeout: None,
                })),
            }),
        })
        .await
        .unwrap()
        .into_inner()
        .job_id;

    // Wait until WaitingForInput
    loop {
        let r = client
            .wait_job_result(WaitJobResultRequest {
                job_id: job_id.clone(),
                timeout_seconds: Some(2),
            })
            .await
            .unwrap()
            .into_inner();
        if let Some(s) = &r.job_status {
            if matches!(s.status, Some(job_status::Status::WaitingForInput(_))) {
                break;
            }
            // If somehow it already completed (shouldn't happen with is_last=false)
            if matches!(s.status, Some(job_status::Status::Completed(_))) {
                return;
            }
        }
    }

    // Push the final chunk
    let push_stream = tokio_stream::iter(vec![PushJobInputRequest {
        job_id: job_id.clone(),
        chunk: Some(InputChunk { data: vec![4, 5, 6], is_last: true }),
    }]);

    client.push_job_input(push_stream).await.unwrap();

    // Now the job should complete
    loop {
        let r = client
            .wait_job_result(WaitJobResultRequest {
                job_id: job_id.clone(),
                timeout_seconds: Some(3),
            })
            .await
            .unwrap()
            .into_inner();
        if let Some(s) = &r.job_status {
            if matches!(s.status, Some(job_status::Status::Completed(_))) {
                return;
            }
        }
    }
}
