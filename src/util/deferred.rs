use core::fmt;
use std::{any::Any, mem, ptr::NonNull};

use autoken::cap;
use derive_where::derive_where;
use rustc_hash::FxHashMap;

use super::arena::{ComponentList, Universe};

cap! {
    pub DeferQueueCap = DeferQueue;
}

#[derive(Default)]
pub struct DeferQueue {
    handlers: FxHashMap<usize, Box<dyn TypedDeferQueue>>,
}

impl fmt::Debug for DeferQueue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DeferQueue").finish_non_exhaustive()
    }
}

trait TypedDeferQueue: Any {
    fn run(&mut self, universe: &Universe);

    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: 'static> TypedDeferQueue for (Deferred<T>, Vec<T>) {
    fn run(&mut self, universe: &Universe) {
        (self.0.handler)(universe, &mut self.1);
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl DeferQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push<T: 'static>(&mut self, handler: Deferred<T>, event: T) {
        self.handlers
            .entry(handler.as_fn() as usize)
            .or_insert_with(|| Box::new((handler, Vec::<T>::new())) as Box<dyn TypedDeferQueue>)
            .as_any_mut()
            .downcast_mut::<(Deferred<T>, Vec<T>)>()
            .unwrap()
            .1
            .push(event);
    }

    pub fn run(&mut self, universe: &Universe) {
        for handler in self.handlers.values_mut() {
            handler.run(universe);
        }
    }
}

#[derive_where(Debug, Copy, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct Deferred<T> {
    handler: fn(&Universe, &mut Vec<T>),
}

impl<T> Deferred<T> {
    pub const fn new<L, F>(f: F) -> Self
    where
        L: ComponentList,
        F: 'static + Fn(T) + Send + Sync,
    {
        assert!(mem::size_of::<F>() == 0);
        mem::forget(f);

        Self {
            handler: |universe, events| {
                let f = unsafe { NonNull::<F>::dangling().as_ref() };
                universe.run::<L, _>(|| {
                    for event in events.drain(..) {
                        f(event);
                    }
                });
            },
        }
    }

    pub fn as_fn(self) -> fn(&Universe, &mut Vec<T>) {
        self.handler
    }

    pub fn run_now(self, universe: &Universe, events: &mut Vec<T>) {
        (self.handler)(universe, events)
    }

    pub fn queue_run(self, event: T)
    where
        T: 'static,
    {
        DeferQueueCap::get_mut(|v| v).0.push(self, event);
    }
}
