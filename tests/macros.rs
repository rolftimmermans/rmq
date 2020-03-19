#[macro_export]
macro_rules! cx {
    () => {
        &mut futures_test::task::noop_context()
    };
}

#[macro_export]
macro_rules! msg {
    () => {
        Message::default()
    };

    ($payload:expr) => {
        Message {
            payload: crate::message::Payload::from(vec![$payload]),
            ..Default::default()
        }
    };

    ($payload:expr, group: $group:expr) => {
        Message {
            payload: crate::message::Payload::from(vec![$payload]),
            group: $group,
        }
    };
}

#[macro_export]
macro_rules! delivery {
    () => {
        Delivery::Message(Default::default())
    };

    ($payload:expr) => {
        Delivery::Message(Message {
            payload: crate::message::Payload::from(vec![$payload]),
            ..Default::default()
        })
    };

    ($payload:expr, group: $group:expr) => {
        Delivery::Message(Message {
            payload: crate::message::Payload::from(vec![$payload]),
            group: $group,
        })
    };
}

#[macro_export]
macro_rules! envelope {
    () => {
        Envelope::default()
    };

    ($payload:expr, $route:expr) => {
        Envelope {
            info: Default::default(),
            route: $route,
            message: Message {
                payload: crate::message::Payload::from(vec![$payload]),
                ..Default::default()
            },
        }
    };
}

#[macro_export]
macro_rules! map {
    () => {
        Default::default()
    };

    ($($key:expr => $value:expr),*) => {
        {
            let mut map = std::collections::HashMap::new();
            $(
                map.insert((&$key[..]).into(), (&$value[..]).into());
            )*
            map
        }
    };
}

#[macro_export]
macro_rules! set {
    () => {
        Default::default()
    };

    ($($value:expr),*) => {
        {
            let mut set = std::collections::HashSet::new();
            $(
                set.insert(($value).into());
            )*
            set
        }
    };
}

#[macro_export]
macro_rules! subscribe_tracing {
    () => {
        let _ = tracing::dispatcher::set_global_default(tracing::dispatcher::Dispatch::new(
            tracing_subscriber::fmt::Subscriber::builder()
                .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
                .finish(),
        ));
    };
}
