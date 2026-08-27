#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::routing::post;
    use axum::Router;
    use goodie_never_deliver::app::*;
    use goodie_never_deliver::ratelimit::{throttle_sign_in, LoginLimiter};
    use leptos::logging::log;
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use sqlx::postgres::PgPoolOptions;

    let conf = get_configuration(None).unwrap();
    let addr = conf.leptos_options.site_addr;
    let leptos_options = conf.leptos_options;
    // Generate the list of routes in your Leptos App
    let routes = generate_route_list(App);

    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL is not set — run `pg-start` inside the nix dev shell");
    let pool = PgPoolOptions::new()
        .max_connections(8)
        .connect(&database_url)
        .await
        .expect("could not connect to Postgres");

    // Schema and seed data both live in ./migrations, so a fresh database is
    // one `pg-start` away from a full catalogue.
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations failed");
    log!("catalogue ready at {database_url}");

    // Creates or updates the admin named by ADMIN_EMAIL / ADMIN_PASSWORD; does
    // nothing when they are unset, so production can opt out entirely.
    match goodie_never_deliver::auth::bootstrap_admin(&pool).await {
        Ok(Some(email)) => log!("admin account ready: {email}"),
        Ok(None) => log!("no ADMIN_EMAIL / ADMIN_PASSWORD set — skipping admin bootstrap"),
        Err(e) => panic!("admin bootstrap failed: {e}"),
    }

    // Server functions need the pool in context twice over: once for the
    // renderer, which calls them directly during SSR, and once for the POST
    // handler the hydrated client talks to.
    let context_pool = pool.clone();
    let provide_pool = move || provide_context(context_pool.clone());
    let provide_pool_fallback = provide_pool.clone();

    // Guessing a password is cheap to attempt and expensive to check, so
    // sign-in gets a per-address budget. See `ratelimit`.
    let limiter = LoginLimiter::new();

    let app =
        Router::new()
            .route(
                "/api/{*fn_name}",
                post({
                    let provide_pool = provide_pool.clone();
                    move |req: axum::extract::Request| {
                        let provide_pool = provide_pool.clone();
                        async move {
                            leptos_axum::handle_server_fns_with_context(provide_pool, req).await
                        }
                    }
                }),
            )
            .leptos_routes_with_context(&leptos_options, routes, provide_pool, {
                let leptos_options = leptos_options.clone();
                move || shell(leptos_options.clone())
            })
            // The fallback renders the app too — without the pool it panics on the
            // catalogue resource instead of serving a 404.
            .fallback(leptos_axum::file_and_error_handler_with_context(
                provide_pool_fallback,
                shell,
            ))
            // Applied **last, on purpose**. Placed straight after the `/api`
            // route it silently never runs — verified by probe: the middleware
            // was not entered for any request. Wrapping the finished router is
            // what works, so every request passes through it and
            // `throttle_sign_in` narrows to `/api/sign_in` itself.
            .layer(axum::middleware::from_fn_with_state(
                limiter,
                throttle_sign_in,
            ))
            .with_state(leptos_options);

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    log!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    // `into_make_service` alone would leave `ConnectInfo<SocketAddr>`
    // unavailable, and the rate limiter has no address to charge.
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await
    .unwrap();
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for pure client-side testing
    // see lib.rs for hydration function instead
}
