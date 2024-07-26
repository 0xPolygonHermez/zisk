use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

pub trait CalculateProver<O, OR> {
    fn calculate(&self, operation: O) -> OR;

    fn prove(&mut self, operations: &[O]);

    fn calculate_prove(&mut self, operation: O) -> OR;
}
pub trait Sessionable {
    fn when_closed(&self);
    fn terminate(&self);
}

pub type SessionId = usize;
type ComponentId = usize;

struct ComponentSessions {
    sessions: HashMap<SessionId, bool>,
    component: Arc<dyn Sessionable>,
}

impl ComponentSessions {
    pub fn new(component: Arc<dyn Sessionable>) -> Self {
        Self { sessions: HashMap::new(), component }
    }
}

pub struct SessionsInner {
    component_sessions: HashMap<ComponentId, ComponentSessions>,
    session_map: HashMap<SessionId, ComponentId>,
}

pub struct Sessions {
    inner: Mutex<SessionsInner>,
}

impl Default for Sessions {
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl Send for Sessions {}
unsafe impl Sync for Sessions {}

impl Sessions {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(SessionsInner {
                component_sessions: HashMap::new(),
                session_map: HashMap::new(),
            }),
        }
    }

    fn get_unique_id(arc: &Arc<dyn Sessionable>) -> ComponentId {
        Arc::as_ptr(arc) as *const () as usize
    }

    pub fn open_session(&self, component: Arc<dyn Sessionable>) -> SessionId {
        let mut inner = self.inner.lock().unwrap();

        let session_id = inner.session_map.len();
        let component_id = Self::get_unique_id(&component);

        let component_session = inner
            .component_sessions
            .entry(component_id)
            .or_insert_with(|| ComponentSessions::new(component.clone()));

        component_session.sessions.insert(session_id, true);

        inner.session_map.insert(session_id, component_id);
        println!("Opened session {}", session_id);
        session_id
    }

    pub fn close_session(&self, session_id: SessionId) -> Result<(), Box<dyn std::error::Error>> {
        println!("Closing session {}", session_id);
        let mut inner = self.inner.lock().unwrap();

        let component_id = match inner.session_map.get(&session_id) {
            Some(&id) => id,
            None => return Err(format!("Session {} not found", session_id).into()),
        };

        let component_sessions = match inner.component_sessions.get_mut(&component_id) {
            Some(sessions) => sessions,
            None => return Err(format!("Component {} not found", component_id).into()),
        };

        if !component_sessions.sessions.contains_key(&session_id) {
            return Err(format!("Session {} is not found", session_id).into());
        }

        component_sessions
            .sessions
            .remove(&session_id)
            .ok_or_else(|| format!("Session {} is already closed", session_id))?;

        // Check if `when_closed` needs to be called
        let should_call_when_closed = component_sessions.sessions.is_empty();

        // Optionally clone the component if `when_closed` will be called
        let component =
            if should_call_when_closed { Some(component_sessions.component.clone()) } else { None };

        // Drop the lock by explicitly dropping `inner`
        drop(inner);

        // Call `when_closed` outside the lock
        if let Some(component) = component {
            component.when_closed();
        }

        Ok(())
    }
}
