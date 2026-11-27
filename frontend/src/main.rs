mod api;
mod auth;
mod components;
mod pages;
mod token;
mod types;
mod utils;

use leptos::prelude::*;
use leptos_router::components::{Route, Router, Routes};
use leptos_router::path;

use components::Layout;
use pages::auth_pages::{LoginPage, RegisterPage};
use pages::community_create::CommunityCreatePage;
use pages::community_detail::CommunityDetailPage;
use pages::community_list::CommunityListPage;
use pages::event_create::EventCreatePage;
use pages::my_communities::MyCommunitiesPage;

#[component]
fn App() -> impl IntoView {
    auth::provide_auth();

    view! {
        <Router>
            <Layout>
                <Routes fallback=|| view! { <p>"ページが見つかりません"</p> }>
                    <Route path=path!("/") view=CommunityListPage />
                    <Route path=path!("/login") view=LoginPage />
                    <Route path=path!("/register") view=RegisterPage />
                    <Route path=path!("/communities/new") view=CommunityCreatePage />
                    <Route path=path!("/communities/:id/events/new") view=EventCreatePage />
                    <Route path=path!("/communities/:id") view=CommunityDetailPage />
                    <Route path=path!("/my-communities") view=MyCommunitiesPage />
                </Routes>
            </Layout>
        </Router>
    }
}

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(App);
}
