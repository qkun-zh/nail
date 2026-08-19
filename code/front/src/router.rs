use leptos::prelude::*;
use leptos_router::components::{Outlet, ParentRoute, Route, Router, Routes};
use leptos_router::path;

use crate::page::article::apply_tag::ApplyTag;
use crate::page::article::create::CreateArticle;
use crate::page::article::delete::DeleteArticle;
use crate::page::article::detail::ArticleDetail;
use crate::page::article::search::Search;
use crate::page::article::unapply_tag::UnapplyTag;
use crate::page::article::undelete_soft::UndeleteSoftArticle;
use crate::page::article::update::UpdateArticle;
use crate::page::article::version::comment::CommentSection;
use crate::page::article::version::create::CreateVersion;
use crate::page::article::version::delete::DeleteVersion;
use crate::page::article::version::detail::VersionDetail;
use crate::page::article::version::index::VersionList;
use crate::page::article::version::undelete_soft::UndeleteSoftVersion;
use crate::page::article::version::update::UpdateVersion;
use crate::page::authenticate::Authenticate;
use crate::page::index::Index;
use crate::page::not_found::NotFound;
use crate::page::role::create::CreateRole;
use crate::page::role::delete::DeleteRole;
use crate::page::role::detail::RoleDetail;
use crate::page::role::list::RoleList;
use crate::page::role::update::UpdateRole;
use crate::page::tag::create::CreateTag;
use crate::page::tag::delete::DeleteTag;
use crate::page::tag::detail::TagDetail;
use crate::page::tag::list::TagList;
use crate::page::tag::update::UpdateTag;
use crate::page::user::article::UserArticle;
use crate::page::user::deregister::Deregister;
use crate::page::user::email::EmailIndex;
use crate::page::user::email::update::EmailUpdate;
use crate::page::user::hub::UserHub;
use crate::page::user::id::UserId;
use crate::page::user::list::UserList;
use crate::page::user::logout::Logout;
use crate::page::user::name::Name;
use crate::page::user::name::update::NameUpdate;
use crate::page::user::role::UserRole;
use crate::page::user::undelete_soft::UndeleteSoftUser;

#[component]
pub fn AppRouter() -> impl IntoView {
    view! {
        <Router>
            <Routes fallback=NotFound>
                <Route path=path!("") view=Index/>
                <Route path=path!("/authenticate") view=Authenticate/>
                <Route path=path!("/search") view=Search/>
                <Route path=path!("/user") view=UserList/>
                <ParentRoute path=path!("/user/:uid") view=Outlet>
                    <Route path=path!("") view=UserHub/>
                    <Route path=path!("/id") view=UserId/>
                    <ParentRoute path=path!("/name") view=Outlet>
                        <Route path=path!("") view=Name/>
                        <Route path=path!("/update") view=NameUpdate/>
                    </ParentRoute>
                    <ParentRoute path=path!("/email") view=Outlet>
                        <Route path=path!("") view=EmailIndex/>
                        <Route path=path!("/update") view=EmailUpdate/>
                    </ParentRoute>
                    <Route path=path!("/role") view=UserRole/>
                    <Route path=path!("/article") view=UserArticle/>
                    <Route path=path!("/logout") view=Logout/>
                    <Route path=path!("/deregister") view=Deregister/>
                    <Route path=path!("/undelete-soft") view=UndeleteSoftUser/>
                </ParentRoute>
                <Route path=path!("/article/create") view=CreateArticle/>
                <ParentRoute path=path!("/article/:article_id") view=Outlet>
                    <Route path=path!("") view=ArticleDetail/>
                    <Route path=path!("/update") view=UpdateArticle/>
                    <Route path=path!("/delete") view=DeleteArticle/>
                    <Route path=path!("/undelete-soft") view=UndeleteSoftArticle/>
                    <Route path=path!("/tag/:tag_id/apply") view=ApplyTag/>
                    <Route path=path!("/tag/:tag_id/unapply") view=UnapplyTag/>
                    <ParentRoute path=path!("/version") view=Outlet>
                        <Route path=path!("") view=VersionList/>
                        <Route path=path!("/create") view=CreateVersion/>
                        <ParentRoute path=path!("/:version_id") view=Outlet>
                            <Route path=path!("") view=VersionDetail/>
                            <Route path=path!("/update") view=UpdateVersion/>
                            <Route path=path!("/delete") view=DeleteVersion/>
                            <Route path=path!("/undelete-soft") view=UndeleteSoftVersion/>
                            <Route
                                path=path!("/*comment_path")
                                view=CommentSection
                            />
                        </ParentRoute>
                    </ParentRoute>
                </ParentRoute>
                <Route path=path!("/tag/create") view=CreateTag/>
                <ParentRoute path=path!("/tag/:tag_id") view=Outlet>
                    <Route path=path!("") view=TagDetail/>
                    <Route path=path!("/update") view=UpdateTag/>
                    <Route path=path!("/delete") view=DeleteTag/>
                </ParentRoute>
                <Route path=path!("/tag") view=TagList/>
                <Route path=path!("/role/create") view=CreateRole/>
                <ParentRoute path=path!("/role/:role_id") view=Outlet>
                    <Route path=path!("") view=RoleDetail/>
                    <Route path=path!("/update") view=UpdateRole/>
                    <Route path=path!("/delete") view=DeleteRole/>
                </ParentRoute>
                <Route path=path!("/role") view=RoleList/>
            </Routes>
        </Router>
    }
}
