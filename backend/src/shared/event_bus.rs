use std::sync::RwLock;

type Handler<E> = Box<dyn Fn(&E) + Send + Sync>;

/// インメモリのイベントバス（コンテキスト間の副作用用）。
pub struct EventBus<E> {
    handlers: RwLock<Vec<Handler<E>>>,
}

impl<E> Default for EventBus<E> {
    fn default() -> Self {
        Self {
            handlers: RwLock::new(Vec::new()),
        }
    }
}

impl<E> EventBus<E> {
    pub fn subscribe(&self, handler: impl Fn(&E) + Send + Sync + 'static) {
        if let Ok(mut handlers) = self.handlers.write() {
            handlers.push(Box::new(handler));
        }
    }

    pub fn publish(&self, event: &E) {
        if let Ok(handlers) = self.handlers.read() {
            for handler in handlers.iter() {
                handler(event);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use meetup_test_macros::describe;
    use std::sync::{Arc, Mutex};

    describe!("イベントバス", {
        test!("購読者がいなくても発行できる", {
            EventBus::<u32>::default().publish(&1);
        });

        test!(
            "複数の購読者が登録順にイベントを受け取る",
            {
                let bus = EventBus::<u32>::default();
                let log = Arc::new(Mutex::new(Vec::new()));
                for name in ["a", "b"] {
                    let log = Arc::clone(&log);
                    bus.subscribe(move |e| log.lock().unwrap().push(format!("{name}{e}")));
                }
                bus.publish(&1);
                bus.publish(&2);
                assert_eq!(*log.lock().unwrap(), vec!["a1", "b1", "a2", "b2"]);
            }
        );
    });
}
