//! Window swallowing in the style of XMonad.Hooks.WindowSwallowing
//!
//! When a client window is opened from a parent that matches a given query, its takes
//! over the parent window's position in the Stack. When the child window closes, the
//! parent is restored in its place.
use crate::{
    core::{hooks::EventHook, State},
    pure::{geometry::Rect, Stack},
    x::{Query, XConn, XConnExt, XEvent},
    Result, Xid,
};
use std::collections::HashMap;
use tracing::{info, warn};

// Private internal state for managing swallowed windows
#[derive(Default, Debug)]
struct WindowSwallowingState {
    swallowed: HashMap<Xid, Xid>, // map of child windows to their swallowed parent
    stack_before_close: Option<Stack<Xid>>,
    floating_before_close: HashMap<Xid, Rect>,
}

impl WindowSwallowingState {
    fn stash_state<X: XConn>(&mut self, state: &mut State<X>) -> Result<bool> {
        self.stack_before_close = state.client_set.current_stack().cloned();
        self.floating_before_close = state.client_set.floating.clone();

        Ok(true)
    }

    fn clear_state_for(&mut self, id: Xid) {
        self.swallowed.remove(&id);
        self.stack_before_close = None;
    }

    fn try_restore_parent<X: XConn>(
        &mut self,
        child: Xid,
        state: &mut State<X>,
        x: &X,
    ) -> Result<bool> {
        warn!(%child, ?self, "checking if we need to restore");
        let parent = match self.swallowed.get(&child) {
            Some(&parent) => parent,
            None => return Ok(true),
        };

        info!(%parent, %child, "destroyed window has a swallowed parent: restoring");
        let len = state.client_set.current_workspace().clients().count();

        let mut old_stack = match self.stack_before_close.take() {
            Some(s) if s.len() - 1 == len && s.focus == child => s,

            // Wrong number of clients or the child is not the focus so we failed to correctly
            // stash the state we need to restore the parent in the correct position. Just
            // re-insert it into the stack and clear our internal state.
            _ => {
                warn!(%parent, %child, "stashed state was invalid: inserting parent directly");
                state.client_set.insert(parent);
                self.clear_state_for(child);

                return Ok(true);
            }
        };

        info!(%parent, %child, "restoring swallowed parent in place of child");
        transfer_floating_state(child, parent, &mut self.floating_before_close);
        state.client_set.floating = self.floating_before_close.clone();
        old_stack.focus = parent;
        state.client_set.modify_occupied(|_| old_stack);
        x.refresh(state)?;
        self.clear_state_for(child);

        Ok(false)
    }
}

pub struct WindowSwallowing<X: XConn> {
    parent: Box<dyn Query<X>>,
    child: Option<Box<dyn Query<X>>>,
}

impl<X: XConn> WindowSwallowing<X> {
    pub fn boxed<Q>(parent: Q) -> Box<dyn EventHook<X>>
    where
        X: 'static,
        Q: Query<X> + 'static,
    {
        Box::new(Self {
            parent: Box::new(parent),
            child: None,
        })
    }

    fn queries_hold(&self, id: Xid, parent: Xid, x: &X) -> bool {
        let parent_matches = x.query_or(false, &*self.parent, parent);
        let child_matches = match &self.child {
            Some(q) => x.query_or(false, &**q, id),
            None => true,
        };

        parent_matches && child_matches
    }

    fn handle_map_request(
        &mut self,
        child: Xid,
        wss: &mut WindowSwallowingState,
        state: &mut State<X>,
        x: &X,
    ) -> Result<bool> {
        let parent = match state.client_set.current_client() {
            Some(&parent) => parent,
            None => return Ok(true), // No parent currently so run default handling
        };

        if !self.queries_hold(child, parent, x) || !is_child_of(child, parent, x) {
            return Ok(true);
        }

        info!(%parent, %child, "matched queries for window swallowing");

        // Set the new window as focus, replacing the parent window.
        wss.swallowed.insert(child, parent);
        state.client_set.modify_occupied(|mut s| {
            s.focus = child;
            s
        });

        // If the parent was floating, copy that state to the child.
        transfer_floating_state(parent, child, &mut state.client_set.floating);

        Ok(false)
    }
}

impl<X: XConn> EventHook<X> for WindowSwallowing<X> {
    fn call(&mut self, event: &XEvent, state: &mut State<X>, x: &X) -> Result<bool> {
        let _wss = state.extension_or_default::<WindowSwallowingState>();
        let mut wss = _wss.borrow_mut();

        match *event {
            // We intercept map requests for windows matching our child query if the
            // currently focused window matches the parent query. If we're unable to
            // pull the _NET_WM_PID property for either window we bail on trying to
            // handle the new window and let the default handling deal with it.
            // NOTE: This does _not_ trigger any user specified manage hooks.
            XEvent::MapRequest(id) => self.handle_map_request(id, &mut wss, state, x),

            // Stash state in case this is before a window closing. If the closed window
            // is one we care about then the stack ordering and any floating position will
            // have been trashed and we need to restore it in try_restore_parent.
            XEvent::ConfigureRequest(_) => wss.stash_state(state),

            // If the destroyed window is a child of one we swallowed then we restore the
            // parent in its place.
            XEvent::Destroy(id) => wss.try_restore_parent(id, state, x),

            // Anything else just gets the default handling from core
            _ => Ok(true),
        }
    }
}

fn transfer_floating_state(from: Xid, to: Xid, floating: &mut HashMap<Xid, Rect>) {
    if let Some(r) = floating.remove(&from) {
        floating.insert(to, r);
    }
}

fn is_child_of<X: XConn>(id: Xid, parent: Xid, x: &X) -> bool {
    match (x.window_pid(parent), x.window_pid(id)) {
        (Some(p_pid), Some(c_pid)) => parent_pid_chain(c_pid).contains(&p_pid),
        _ => false,
    }
}

// Parsing based on the format for /proc/pid/stat in https://man.archlinux.org/man/proc.5
// This will bottom out when the parent pid hits root (0) due to there being no stat file for root
fn parent_pid(pid: u32) -> Option<u32> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let s_parent_pid = stat.split_whitespace().nth(3).expect("/proc to be valid");

    s_parent_pid.parse().ok()
}

fn parent_pid_chain(mut pid: u32) -> Vec<u32> {
    let mut parents = vec![];

    while let Some(parent) = parent_pid(pid) {
        parents.push(parent);
        pid = parent;
    }

    parents
}
