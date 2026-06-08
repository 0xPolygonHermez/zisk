use super::{
    classify_worker_error, WORKER_ERROR_AGG_FAIL, WORKER_ERROR_CHANNEL_CLOSED,
    WORKER_ERROR_HEARTBEAT_LOST, WORKER_ERROR_PROVE_FAIL, WORKER_ERROR_SETUP_FAIL,
    WORKER_ERROR_UNKNOWN, WORKER_ERROR_UNREACHABLE,
};

#[test]
fn worker_error_classifier_is_bounded() {
    assert_eq!(
        classify_worker_error("[Monitor] Worker w0 missed heartbeats for job j1"),
        WORKER_ERROR_HEARTBEAT_LOST
    );
    assert_eq!(
        classify_worker_error("Failed to send message: channel closed"),
        WORKER_ERROR_CHANNEL_CLOSED
    );
    assert_eq!(
        classify_worker_error("all workers unreachable during setup"),
        WORKER_ERROR_UNREACHABLE
    );
    assert_eq!(
        classify_worker_error("Failed to send SetupProgram to worker"),
        WORKER_ERROR_SETUP_FAIL
    );
    assert_eq!(classify_worker_error("aggregate step failed in worker"), WORKER_ERROR_AGG_FAIL);
    assert_eq!(classify_worker_error("prove phase 2 failure in worker"), WORKER_ERROR_PROVE_FAIL);
    assert_eq!(classify_worker_error("unexpected internal worker error 42"), WORKER_ERROR_UNKNOWN);
}
