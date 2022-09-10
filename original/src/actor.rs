//! A simple, lightweight actor framework for internal use
use crossbeam_channel::{Receiver, Sender};
use tracing::error;

// TODO: the Send/Recv error variants here probably need better names
/// An error encountered while interacting with an Actor
#[derive(Debug, thiserror::Error)]
pub enum Error<E> {
    /// An error was returned from the Actor while resolving the [`Request`]
    #[error("error while running resolver")]
    Resolve(E),
    /// The channel for communicating with the Actor is closed
    #[error("attempt to send on closed channel to the Actor")]
    Send,
    /// The channel for receiving the response from the Actor is closed
    #[error("actor closed the response channel")]
    Recv,
}

type Result<R> = std::result::Result<<R as Request>::Response, Error<<R as Request>::Error>>;

/// A request that can be handled by an Actor. For an Actor to be able to
/// resolve a given [`Request`] it must implement [`Resolve`] for it.
pub trait Request: Sized + Send {
    /// The happy path response type
    type Response: Send;
    /// The error path response type
    type Error: Send;
}

/// Register the ability for an Actor to resolve a certain type of [`Request`]
pub trait Resolve<R: Request> {
    // NOTE: the request needs to be a mutable reference here as we are sending it
    //       to the actor as a boxed trait object. Moving _out_ of a box requires
    //       knowing the size of the type being moved which is unknown for a trait
    //       object.

    /// Resolve a request using this Actor
    fn resolve(&mut self, req: &mut R) -> std::result::Result<R::Response, R::Error>;
}

/// A message for an actor
#[derive(Debug)]
pub struct Message<R: Request> {
    /// The [`Request`] being sent
    pub req: R,
    /// A channel for sending back the response
    pub tx: Sender<Result<R>>,
}

/// Message wrapper for a given request type
#[derive(Debug)]
pub enum Wrapper<T> {
    /// A [`Request`] for processing
    Request(T),
    /// Signal that the owner of the communication channel being used should shut down
    ShutDown,
}

/// A handle for submitting Requests for processing
#[derive(Debug)]
pub struct Handle<T> {
    tx: Sender<Wrapper<T>>,
}

// NOTE: implemented by hand to avoid requiring T to be Clone
impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
        }
    }
}

impl<T> Handle<T> {
    /// Send a request to the Actor for it to resolve and return the result
    pub fn resolve<R>(&self, req: R) -> Result<R>
    where
        R: Request + Into<(T, Receiver<Result<R>>)> + 'static,
    {
        let (wrapped, rx) = req.into();

        if self.tx.send(Wrapper::Request(wrapped)).is_err() {
            return Err(Error::Send);
        }

        match rx.recv() {
            Ok(res) => res,
            Err(_) => Err(Error::Recv),
        }
    }

    /// Shut down the associated Actor.
    ///
    /// This is a no-op if the actor has already been shut down.
    pub fn shutdown(self) {
        if self.tx.send(Wrapper::ShutDown).is_err() {
            // channel already closed
        }
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

/// Quickly rovide implementations of [`Resolve`] for a set of [`Request`]
/// types.
#[macro_export]
macro_rules! actor {
    (
        type $actor:ty;
        params: ($self:ident, $r:ident);
        $($req:ty => $body:expr;)+
    ) => {
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

/// Group a set of [`Request`] types together for the purposes of establishing a communication
/// interface.
#[macro_export]
macro_rules! request_set {
    ($vis:vis enum $enum:ident; trait $trait:ident => [ $($req:ident),+ ]) => {
        $vis enum $enum {
            $($req($crate::actor::Message<$req>),)+
        }

        $vis trait $trait: Send + Sized + 'static $(+ $crate::actor::Resolve<$req>)+ {
            fn resolve_enum(&mut self, msg: $enum) {
                match msg {
                    $(
                        $enum::$req($crate::actor::Message { mut req, tx }) => {
                            let res = match self.resolve(&mut req) {
                                Ok(res) => Ok(res),
                                Err(e) => Err($crate::actor::Error::Resolve(e)),
                            };

                            if tx.send(res).is_err() {
                                // The client dropped their end of the channel
                            }
                        },
                    )+
                }
            }

            fn run_threaded(mut self) -> ($crate::actor::Handle<$enum>, std::thread::JoinHandle<()>) {
                let (tx, rx) = crossbeam_channel::unbounded::<$crate::actor::Wrapper<$enum>>();

                let handle = std::thread::spawn(move || loop {
                    let w = match rx.recv() {
                        Ok(w) => w,
                        Err(_) => break, // channel is now disconnected
                    };

                    match w {
                       $crate::actor::Wrapper::Request(req) => self.resolve_enum(req),
                       $crate::actor::Wrapper::ShutDown => break,
                    }
                });

                ($crate::actor::Handle { tx }, handle)
            }
        }

        $(
            impl From<$req> for ($enum, crossbeam_channel::Receiver<Result<$req>>) {
                fn from(req: $req) -> ($enum, crossbeam_channel::Receiver<Result<$req>>) {
                    let (tx, rx) = crossbeam_channel::bounded(1);

                    ($enum::$req($crate::actor::Message { req, tx }), rx)
                }
            }
        )+

        impl<T> $trait for T where T: Send + Sized + 'static $(+ Resolve<$req>)+ {}
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

    request_set!(enum Example; trait ResolveExample => [Add, Concat, Failure]);

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
        thread_handle.join().unwrap();
        let res = cloned.resolve(Add(1, 2));

        assert!(matches!(res, Err(Error::Send)));
    }
}
