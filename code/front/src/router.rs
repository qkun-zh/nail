use leptos::prelude::*;
use leptos_router::components::{Outlet, ParentRoute, Route, Router, Routes};
use leptos_router::path;

use crate::page::article::create::CreateArticle;
use crate::page::article::delete::DeleteArticle;
use crate::page::article::detail::ArticleDetail;
use crate::page::article::search::Search;
use crate::page::article::update::UpdateArticle;
use crate::page::article::version::comment::CommentSection;
use crate::page::article::version::create::CreateVersion;
use crate::page::article::version::detail::VersionDetail;
use crate::page::article::version::index::VersionList;
use crate::page::authenticate::Authenticate;
use crate::page::index::Index;
use crate::page::not_found::NotFound;
use crate::page::user::deregister::Deregister;
use crate::page::user::email::EmailIndex;
use crate::page::user::email::update::EmailUpdate;
use crate::page::user::hub::UserHub;
use crate::page::user::logout::Logout;
use crate::page::user::name::Name;
use crate::page::user::name::update::NameUpdate;

#[component]
pub fn AppRouter() -> impl IntoView {
    view! {
        <Router>
            <Routes fallback=NotFound>
                <Route path=path!("") view=Index/>
                <Route path=path!("/authenticate") view=Authenticate/>
                <Route path=path!("/search") view=Search/>
                <ParentRoute path=path!("/user/:uid") view=Outlet>
                    <Route path=path!("") view=UserHub/>
                    <Route path=path!("/name") view=Name/>
                    <Route path=path!("/name/update") view=NameUpdate/>
                    <Route path=path!("/email") view=EmailIndex/>
                    <Route path=path!("/email/update") view=EmailUpdate/>
                    <Route path=path!("/logout") view=Logout/>
                    <Route path=path!("/deregister") view=Deregister/>
                </ParentRoute>
                <Route path=path!("/article/create") view=CreateArticle/>
                <Route path=path!("/article/:article_id") view=ArticleDetail/>
                <Route path=path!("/article/:article_id/update") view=UpdateArticle/>
                <Route path=path!("/article/:article_id/delete") view=DeleteArticle/>
                <Route path=path!("/article/:article_id/version") view=VersionList/>
                <Route path=path!("/article/:article_id/version/create") view=CreateVersion/>
                <Route path=path!("/article/:article_id/version/:version_id") view=VersionDetail/>
                <Route
                    path=path!("/article/:article_id/version/:version_id/*comment_path")
                    view=CommentSection
                />
            </Routes>
        </Router>
    }
}
