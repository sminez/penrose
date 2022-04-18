//! A simple, lightweight actor framework for internal use
use crossbeam_channel::{bounded, unbounded, Receiver, Sender};
use std::thread::{self, JoinHandle};
use tracing::error;

// TODO: the Send/Recv error variants here probably need better names
/// An error encountered while interacting with an [`Actor`]
#[derive(Debug, thiserror::Error)]
pub enum Error<E> {
    /// An error was returned from the [`Actor`] while resolving the [`Request`]
    #[error("error while running resolver")]
    Resolve(E),
    /// The channel for communicating with the [`Actor`] is closed
    #[error("attempt to send on closed channel to the Actor")]
    Send,
    /// The channel for receiving the response from the [`Actor`] is closed
    #[error("actor closed the response channel")]
    Recv,
}

type Result<R> = std::result::Result<<R as Request>::Response, Error<<R as Request>::Error>>;

/// A lightweight actor for handling messages
pub trait Actor: Send + Sized + 'static {
    /// Run this Actor in its own thread until it is shut down, returning an
    /// [`ActorHandle`] for sending requests to it. This is not the only way
    /// to run an Actor but it is provided as a convenient way to handle the
    /// lifecycle of the communication channels involved.
    fn run_threaded(mut self) -> (ActorHandle<Self>, JoinHandle<()>) {
        let (tx, rx) = unbounded::<Wrapper<Self>>();

        let handle = thread::spawn(move || loop {
            let mut w = match rx.recv() {
                Ok(w) => w,
                Err(_) => break, // channel is now disconnected
            };

            match w {
                Wrapper::Request(ref mut req) => req.resolve(&mut self),
                Wrapper::ShutDown => break,
            }
        });

        (ActorHandle { tx }, handle)
    }
}

/// A handle for communicating with a running [`Actor`]
#[derive(Debug)]
pub struct ActorHandle<A: Actor> {
    tx: Sender<Wrapper<A>>,
}

impl<A: Actor> Clone for ActorHandle<A> {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
        }
    }
}

impl<A: Actor> ActorHandle<A> {
    /// Send a request to the [`Actor`] for it to resolve and return the result
    pub fn resolve<R>(&self, req: R) -> Result<R>
    where
        R: Request + 'static,
        A: Resolve<R>,
    {
        let (wrapped, rx) = Wrapper::message(req);

        if let Err(_) = self.tx.send(wrapped) {
            return Err(Error::Send);
        }

        match rx.recv() {
            Ok(res) => res,
            Err(_) => Err(Error::Recv),
        }
    }

    /// Shut down the associated [`Actor`].
    ///
    /// This is a no-op if the actor has already been shut down.
    pub fn shutdown(self) {
        if let Err(_) = self.tx.send(Wrapper::ShutDown) {
            // channel already closed
            return;
        }
    }
}

/// A request that can be handled by an [`Actor`]. For an Actor to be able to
/// resolve a given [`Request`] it must implement [`Resolve`] for it.
pub trait Request: Send {
    /// The happy path response type
    type Response: Send;
    /// The error path response type
    type Error: Send;
}

/// Register the ability for an [`Actor`] to resolve a certain type of [`Request`]
pub trait Resolve<R: Request>: Actor {
    // NOTE: the request needs to be a mutable reference here as we are sending it
    //       to the actor as a boxed trait object. Moving _out_ of a box requires
    //       knowing the size of the type being moved which is unknown for a trait
    //       object.

    /// Resolve a request using this [`Actor`]
    fn resolve(&mut self, req: &mut R) -> std::result::Result<R::Response, R::Error>;
}

pub(crate) struct Message<R: Request> {
    req: R,
    tx: Sender<Result<R>>,
}

pub(crate) trait Proxy<A: Actor> {
    fn resolve(&mut self, actor: &mut A);
}

impl<A, R> Proxy<A> for Message<R>
where
    A: Resolve<R>,
    R: Request,
{
    fn resolve(&mut self, actor: &mut A) {
        let Message { req, tx } = self;
        let res = match actor.resolve(req) {
            Ok(res) => Ok(res),
            Err(e) => Err(Error::Resolve(e)),
        };

        if let Err(_) = tx.send(res) {
            error!("unable to send response when resolving Actor request");
            return;
        }
    }
}

pub(crate) enum Wrapper<A: Actor> {
    Request(Box<dyn Proxy<A> + Send>),
    ShutDown,
}

impl<A: Actor> Wrapper<A> {
    fn message<R>(req: R) -> (Self, Receiver<Result<R>>)
    where
        R: Request + 'static,
        A: Resolve<R>,
    {
        let (tx, rx) = bounded(1);
        let msg = Message { req, tx };
        let w = Wrapper::Request(Box::new(msg));

        (w, rx)
    }
}

/// Quickly implement [`Request`] for a given type
#[macro_export]
macro_rules! request {
    ($req:ident => $res:ty) => {
        request!($req => $res, ());
    };

    ($req:ident => $res:ty, $err:ty) => {
        impl Request for $req {
            type Response = $res;
            type Error = $err;
        }
    };
}

/// Quickly implement [`Actor`] for a given type and provide implementations of
/// [`Resolve`] for a set of [`Request`] types.
#[macro_export]
macro_rules! actor {
	(
        type $actor:ty;
        params: ($self:ident, $r:ident);
        $($req:ty => $body:expr;)+
    ) => {
		impl Actor for $actor {}

        $(
            impl Resolve<$req> for $actor {
                fn resolve(
                    &mut $self,
                    $r: &mut $req
                ) -> std::result::Result<
                    <$req as Request>::Response,
                    <$req as Request>::Error
                > {
                    $body
                }
            }
        )+
	};
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Add(usize, usize);
    request!(Add => usize);

    struct Concat(&'static str, &'static str);
    request!(Concat => String);

    struct Failure(bool);
    request!(Failure => &'static str, &'static str);

    struct TestActor;
    actor! {
        type TestActor;
        params: (self, req);

        Add => Ok(req.0 + req.1);
        Concat => Ok(format!("{}{}", req.0, req.1));
        Failure => {
            if req.0 {
                Err("failed")
            } else {
                Ok("succeeded")
            }
        };
    }

    #[test]
    fn requests_work() {
        let a = TestActor;
        let (handle, _) = a.run_threaded();

        let res = handle.resolve(Add(1, 2)).unwrap();
        assert_eq!(res, 3);

        let res = handle.resolve(Concat("hello,", " world!")).unwrap();
        assert_eq!(res, "hello, world!");
    }

    #[test]
    fn errors_are_returned_correctly() {
        let a = TestActor;
        let (handle, _) = a.run_threaded();

        let res = handle.resolve(Failure(false));
        assert!(res.is_ok());

        let res = handle.resolve(Failure(true));
        assert!(matches!(res, Err(Error::Resolve("failed"))));
    }

    #[test]
    fn send_to_actor_after_shutdown_is_an_error() {
        let a = TestActor;
        let (handle, thread_handle) = a.run_threaded();
        let cloned = handle.clone();

        handle.shutdown();
        thread_handle.join().expect("thread to exit");
        let res = cloned.resolve(Add(1, 2));

        assert!(matches!(res, Err(Error::Send)));
    }
}
