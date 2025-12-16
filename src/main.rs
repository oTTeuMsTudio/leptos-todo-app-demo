use crate::app::*;
use axum::Router;
use leptos::{config::get_configuration, logging};
use leptos_axum::{generate_route_list, LeptosRoutes};
use server_fns_axum::*;

#[allow(clippy::needless_return)]
#[tokio::main]
async fn main() {
    simple_logger::init_with_level(log::Level::Error)
        .expect("couldn't initialize logging");

    let conf = get_configuration(None).unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(App);

    let app = Router::new()
        .leptos_routes(&leptos_options, routes, {
            let leptos_options = leptos_options.clone();
            move || shell(leptos_options.clone())
        })
        .fallback(leptos_axum::file_and_error_handler(shell))
        .with_state(leptos_options);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    logging::log!("listening on http://{}", &addr);
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}
