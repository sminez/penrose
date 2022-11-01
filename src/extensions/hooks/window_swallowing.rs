//! Window swallowing in the style of XMonad.Hooks.WindowSwallowing
//!
//! See https://hackage.haskell.org/package/xmonad-contrib-0.17.0/docs/XMonad-Hooks-WindowSwallowing.html
//! for details of what the original Haskell implementation is doing.
use crate::{
    core::{hooks::EventHook, State},
    pure::{geometry::Rect, Stack},
    x::{property::Prop, Query, XConn, XConnExt, XEvent},
    Result, Xid,
};
use std::collections::HashMap;

#[derive(Default)]
struct WindowSwallowingState {
    swallowed: HashMap<Xid, Xid>, // map of child windows to their swallowed parent
    stack_before_close: Option<Stack<Xid>>,
    floating_before_close: HashMap<Xid, Rect>,
}

impl WindowSwallowingState {
    // Stash state in case this is before a window closing. If the closed window
    // is one we care about then the stack ordering and any floating position will
    // have been trashed and we need to restore it in handle_destroy.
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
        let parent = match self.swallowed.get(&child) {
            Some(&parent) => parent,
            None => return Ok(true),
        };

        let len = state.client_set.current_workspace().clients().count();

        let mut old_stack = match self.stack_before_close.take() {
            Some(s) if s.len() - 1 == len && s.focus == child => s,

            // Wrong number of clients or the child is not the focus so we failed to correctly
            // stash the state we need to restore the parent in the correct position. Just
            // re-insert it into the stack and clear our internal state.
            _ => {
                state.client_set.insert(parent);
                self.clear_state_for(parent);

                return Ok(true);
            }
        };

        transfer_floating_state(child, parent, &mut self.floating_before_close);
        state.client_set.floating = self.floating_before_close.clone();
        old_stack.focus = parent;
        state.client_set.modify_occupied(|_| old_stack);
        x.refresh(state)?;
        self.clear_state_for(parent);

        Ok(true)
    }
}

pub struct WindowSwallowing<X: XConn> {
    parent: Box<dyn Query<X>>,
    child: Option<Box<dyn Query<X>>>,
}

impl<X: XConn> WindowSwallowing<X> {
    fn queries_hold(&self, id: Xid, parent: Xid, x: &X) -> bool {
        let parent_matches = x.query_or(false, &*self.parent, parent);
        let child_matches = match &self.child {
            Some(q) => x.query_or(false, &**q, id),
            None => true,
        };

        parent_matches && child_matches
    }

    // We intercept map requests for windows matching our child query if the currently focused window
    // matches the parent query. If we're unable to pull the _NET_WM_PID property for either window
    // we bail on trying to handle the new window and let the default handling deal with it.
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
            XEvent::MapRequest(id) => self.handle_map_request(id, &mut wss, state, x),
            XEvent::ConfigureRequest(_) => wss.stash_state(state),
            XEvent::Destroy(id) => wss.try_restore_parent(id, state, x),
            _ => Ok(true),
        }
    }
}

fn transfer_floating_state(from: Xid, to: Xid, floating: &mut HashMap<Xid, Rect>) {
    if let Some(r) = floating.remove(&from) {
        floating.insert(to, r);
    }
}

fn pid<X: XConn>(id: Xid, x: &X) -> Option<u32> {
    if let Ok(Some(Prop::Cardinal(vals))) = x.get_prop(id, "_NET_WM_PID") {
        Some(vals[0])
    } else {
        None
    }
}

fn is_child_of<X: XConn>(id: Xid, parent: Xid, x: &X) -> bool {
    match (pid(parent, x), pid(id, x)) {
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
