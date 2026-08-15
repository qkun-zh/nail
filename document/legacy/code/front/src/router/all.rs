use crate::page::*;
use leptos::prelude::*;
use leptos_router::components::*;
use leptos_router::path;

#[component]
pub fn All() -> impl IntoView {
    view! {
        <Router>
            <Routes fallback=NotFound>
                <Route path=path!("/") view=Index/>
                <ParentRoute path=path!("/public") view=PublicLayout>
                    <Route path=path!("") view=PublicIndex/>
                    <Route path=path!("/article") view=ArticleIndex/>
                    <Route path=path!("/article/search") view=Search/>
                    <Route path=path!("/article/create") view=CreateArticle/>
                    <Route path=path!("/article/:article_id/update") view=UpdateArticle/>
                    <Route path=path!("/article/:article_id/delete") view=DeleteArticle/>
                    <Route path=path!("/article/:article_id") view=Detail/>
                    <Route path=path!("/article/:article_id/version") view=VersionList/>
                    <Route path=path!("/article/:article_id/version/create") view=CreateVersion/>
                    <Route path=path!("/article/:article_id/version/:version_id/*comment_path") view=Version/>
                </ParentRoute>
                <ParentRoute path=path!("/private") view=PrivateLayout>
                    <Route path=path!("") view=PrivateIndex/>
                    <Route path=path!("/authenticate") view=Authenticate/>
                    <Route path=path!("/name") view=Name/>
                    <Route path=path!("/name/update") view=NameUpdate/>
                    <Route path=path!("/email") view=EmailIndex/>
                    <Route path=path!("/email/update") view=Update/>
                    <Route path=path!("/logout") view=Logout/>
                    <Route path=path!("/deregister") view=Deregister/>
                </ParentRoute>
            </Routes>
        </Router>
    }
}
