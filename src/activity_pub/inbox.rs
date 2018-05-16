use activitystreams_types::{
    actor::Person,
    activity::{Create, Follow, Like, Undo},
    object::{Article, Note}
};
use activitystreams::activity::Activity;
use diesel::PgConnection;
use failure::Error;
use serde_json;

// use activity_pub::broadcast;
use activity_pub::actor::Actor as APActor;
use activity_pub::sign::*;
use models::blogs::Blog;
use models::comments::*;
use models::likes;
use models::posts::*;
use models::users::User;

#[derive(Fail, Debug)]
enum InboxError {
    #[fail(display = "The `type` property is required, but was not present")]
    NoType,
    #[fail(display = "Invalid activity type")]
    InvalidType,
    #[fail(display = "Couldn't undo activity")]
    CantUndo
}

pub trait Inbox {
    fn received(&self, conn: &PgConnection, act: serde_json::Value);

    fn new_article(&self, conn: &PgConnection, article: Article) -> Result<(), Error> {
        Post::insert(conn, NewPost {
            blog_id: 0, // TODO
            slug: String::from(""), // TODO
            title: article.object_props.name_string().unwrap(),
            content: article.object_props.content_string().unwrap(),
            published: true,
            license: String::from("CC-0"),
            ap_url: article.object_props.url_string()?
        });
        Ok(())
    }

    fn new_comment(&self, conn: &PgConnection, note: Note, actor_id: String) -> Result<(), Error> {
        let previous_url = note.object_props.in_reply_to.clone().unwrap().as_str().unwrap().to_string();
        let previous_comment = Comment::find_by_ap_url(conn, previous_url.clone());
        Comment::insert(conn, NewComment {
            content: note.object_props.content_string().unwrap(),
            spoiler_text: note.object_props.summary_string().unwrap(),
            ap_url: note.object_props.id_string().ok(),
            in_response_to_id: previous_comment.clone().map(|c| c.id),
            post_id: previous_comment
                .map(|c| c.post_id)
                .unwrap_or_else(|| Post::find_by_ap_url(conn, previous_url).unwrap().id),
            author_id: User::from_url(conn, actor_id).unwrap().id,
            sensitive: false // "sensitive" is not a standard property, we need to think about how to support it with the activitystreams crate
        });
        Ok(())
    }

    fn follow(&self, conn: &PgConnection, follow: Follow) -> Result<(), Error> {
        let from = User::from_url(conn, follow.actor.as_str().unwrap().to_string()).unwrap();
        match User::from_url(conn, follow.object.as_str().unwrap().to_string()) {
            Some(u) => self.accept_follow(conn, &from, &u, &follow, from.id, u.id),
            None => {
                let blog = Blog::from_url(conn, follow.object.as_str().unwrap().to_string()).unwrap();
                self.accept_follow(conn, &from, &blog, &follow, from.id, blog.id)
            }
        };
        Ok(())
    }

    fn like(&self, conn: &PgConnection, like: Like) -> Result<(), Error> {
        let liker = User::from_url(conn, like.actor.as_str().unwrap().to_string());
        let post = Post::find_by_ap_url(conn, like.object.as_str().unwrap().to_string());
        likes::Like::insert(conn, likes::NewLike {
            post_id: post.unwrap().id,
            user_id: liker.unwrap().id,
            ap_url: like.object_props.id_string()?
        });
        Ok(())
    }

    fn unlike(&self, conn: &PgConnection, undo: Undo) -> Result<(), Error> {
        let like = likes::Like::find_by_ap_url(conn, undo.object_object::<Like>()?.object_props.id_string()?).unwrap();
        like.delete(conn);
        Ok(())
    }

    fn save(&self, conn: &PgConnection, act: serde_json::Value) -> Result<(), Error> {
        match act["type"].as_str() {
            Some(t) => {
                match t {
                    "Create" => {
                        let act: Create = serde_json::from_value(act.clone())?;
                        match act.object["type"].as_str().unwrap() {
                            "Article" => self.new_article(conn, act.object_object().unwrap()),
                            "Note" => self.new_comment(conn, act.object_object().unwrap(), act.actor_object::<Person>()?.object_props.id_string()?),
                            _ => Err(InboxError::InvalidType)?
                        }
                    },
                    "Follow" => self.follow(conn, serde_json::from_value(act.clone())?),
                    "Like" => self.like(conn, serde_json::from_value(act.clone())?),
                    "Undo" => {
                        let act: Undo = serde_json::from_value(act.clone())?;
                        match act.object["type"].as_str().unwrap() {
                            "Like" => self.unlike(conn, act),
                            _ => Err(InboxError::CantUndo)?
                        }
                    }
                    _ => Err(InboxError::InvalidType)?
                }
            },
            None => Err(InboxError::NoType)?
        }
    }

    fn accept_follow<A: Signer, B: Clone, T: Activity>(
        &self,
        _conn: &PgConnection,
        _from: &A,
        _target: &B,
        _follow: &T,
        _from_id: i32,
        _target_id: i32
    ) {
        // TODO
        //Follow::insert(conn, NewFollow {
        //    follower_id: from_id,
        //    following_id: target_id
        //});

        //let accept = activity::Accept::new(target, follow, conn);
        //broadcast(conn, from, accept, vec![target.clone()]);
    }
}
