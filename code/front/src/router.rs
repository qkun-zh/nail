use leptos::prelude::*;
use leptos_router::components::{ParentRoute, Route, Router, Routes};
use leptos_router::path;

use crate::page::index::Index;
use crate::page::not_found::NotFound;
use crate::page::private::authenticate::Authenticate;
use crate::page::private::deregister::Deregister;
use crate::page::private::email::EmailIndex;
use crate::page::private::email::update::EmailUpdate;
use crate::page::private::index::PrivateIndex;
use crate::page::private::logout::Logout;
use crate::page::private::name::Name;
use crate::page::private::name::update::NameUpdate;
use crate::page::public::PublicLayout;
use crate::page::public::article::create::CreateArticle;
use crate::page::public::article::delete::DeleteArticle;
use crate::page::public::article::detail::ArticleDetail;
use crate::page::public::article::index::ArticleIndex;
use crate::page::public::article::update::UpdateArticle;
use crate::page::public::article::version::comment::delete::CommentDelete;
use crate::page::public::article::version::comment::index::CommentIndex;
use crate::page::public::article::version::comment::reply::CommentReply;
use crate::page::public::article::version::create::CreateVersion;
use crate::page::public::article::version::detail::VersionDetail;
use crate::page::public::article::version::index::VersionList;
use crate::page::public::index::PublicIndex;
use crate::page::private::PrivateLayout;
use crate::page::session_gate::RootGate;

#[component]
pub fn AppRouter() -> impl IntoView {
    view! {
        <Router>
            <Routes fallback=NotFound>
                <Route path=path!("/private/authenticate") view=Authenticate/>
                <ParentRoute path=path!("/") view=RootGate>
                    <Route path=path!("") view=Index/>
                    <ParentRoute path=path!("/public") view=PublicLayout>
                        <Route path=path!("") view=PublicIndex/>
                        <Route path=path!("/article") view=ArticleIndex/>
                        <Route path=path!("/article/create") view=CreateArticle/>
                        <Route path=path!("/article/:article_id") view=ArticleDetail/>
                        <Route path=path!("/article/:article_id/update") view=UpdateArticle/>
                        <Route path=path!("/article/:article_id/delete") view=DeleteArticle/>
                        <Route path=path!("/article/:article_id/version") view=VersionList/>
                        <Route path=path!("/article/:article_id/version/create") view=CreateVersion/>
                        <Route path=path!("/article/:article_id/version/:version_id") view=VersionDetail/>
                        <Route
                            path=path!("/article/:article_id/version/:version_id/comment")
                            view=CommentIndex
                        />
                        <Route
                            path=path!("/article/:article_id/version/:version_id/comment/:comment_id")
                            view=CommentReply
                        />
                        <Route
                            path=path!("/article/:article_id/version/:version_id/comment/:comment_id/delete")
                            view=CommentDelete
                        />
                    </ParentRoute>
                    <ParentRoute path=path!("/private") view=PrivateLayout>
                        <Route path=path!("") view=PrivateIndex/>
                        <Route path=path!("/name") view=Name/>
                        <Route path=path!("/name/update") view=NameUpdate/>
                        <Route path=path!("/email") view=EmailIndex/>
                        <Route path=path!("/email/update") view=EmailUpdate/>
                        <Route path=path!("/logout") view=Logout/>
                        <Route path=path!("/deregister") view=Deregister/>
                    </ParentRoute>
                </ParentRoute>
            </Routes>
        </Router>
    }
}
