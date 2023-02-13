package tailcall.gateway

import tailcall.gateway.ast.{Endpoint, Orc}
import tailcall.gateway.remote.Remote
import zio.schema.{DeriveSchema, DynamicValue}
import zio.test._

object OrcSpec extends ZIOSpecDefault {

  object schema {
    final case class User(id: Int, name: String)
    object User    {
      implicit val schema = DeriveSchema.gen[User]
    }
    final case class Post(id: Int, title: String, body: String, userId: Int)
    object Post    {
      implicit val schema = DeriveSchema.gen[Post]
    }
    final case class Comment(id: Int, name: String, email: String, body: String, postId: Int)
    object Comment {
      implicit val schema = DeriveSchema.gen[Comment]
    }
    final case class Album(id: Int, title: String, userId: Int)
    object Album   {
      implicit val schema = DeriveSchema.gen[Album]
    }
    final case class Photo(id: Int, title: String, url: String, thumbnailUrl: String, albumId: Int)
    object Photo   {
      implicit val schema = DeriveSchema.gen[Photo]
    }
    final case class Todo(id: Int, title: String, completed: Boolean, userId: Int)
    object Todo    {
      implicit val schema = DeriveSchema.gen[Todo]
    }
    final case class UserId(userId: Int)
    object UserId  {
      implicit val schema = DeriveSchema.gen[UserId]
    }
    final case class PostId(postId: Int)
    object PostId  {
      implicit val schema = DeriveSchema.gen[PostId]
    }
    final case class EmailId(email: String)
    object EmailId {
      implicit val schema = DeriveSchema.gen[EmailId]
    }
    final case class AlbumId(albumId: Int)
    object AlbumId {
      implicit val schema = DeriveSchema.gen[AlbumId]
    }
  }

  object endpoints {
    import tailcall.gateway.OrcSpec.schema._
    val typicode: Endpoint  = Endpoint.make("jsonplaceholder.typicode.com")
    val users: Endpoint     = typicode.withPath("/users").withOutput[List[User]]
    val posts: Endpoint     = typicode.withPath("/posts").withOutput[List[Post]]
    val userPosts: Endpoint = typicode
      .withPath("/posts")
      .withQuery("userId" -> "${userId}")
      .withInput[UserId]
      .withOutput[List[Post]]

    val postComments: Endpoint = typicode
      .withPath("/comments")
      .withQuery("postId" -> "${postId}")
      .withInput[PostId]
      .withOutput[List[Comment]]

    val userComments: Endpoint = typicode
      .withPath("/comments")
      .withQuery("email" -> "${email}")
      .withInput[EmailId]
      .withOutput[List[Comment]]

    val userAlbums: Endpoint = typicode
      .withPath("/albums")
      .withQuery("userId" -> "${userId}")
      .withInput[UserId]
      .withOutput[List[Album]]

    val UserTodos: Endpoint = typicode
      .withPath("/todos")
      .withQuery("userId" -> "${userId}")
      .withInput[UserId]
      .withOutput[List[Todo]]

    val AlbumPhotos: Endpoint = typicode
      .withPath("/photos")
      .withQuery("albumId" -> "${albumId}")
      .withInput[AlbumId]
      .withOutput[List[Photo]]

    val ManyUsersPosts: Endpoint = typicode
      .withPath("/posts")
      .withQuery("userId" -> "${userId}")
      .withInput[UserId]
      .withOutput[List[Post]]
  }

  val unit: Remote[DynamicValue] = Remote.dynamicValue(())

  val query: Orc = Orc.query(
    "Query" -> List(
      "users" -> (_ => endpoints.users(unit)),
      "posts" -> (_ => endpoints.posts(unit))
    ),
    "User"  -> List(
      "posts"    -> { context => endpoints.userPosts(Remote.record("userId" -> context.value)) },
      /*"manyUsersPosts" -> { context =>
        Remote.batch(
          endpoints.ManyUsersPosts(Remote.record("userId" -> context.value.path("id").getOrDie)),
          List("userId")
        )
      },*/
      "fullName" -> { context =>
        val fn = context.value.path("firstName").flatMap(_.asString).getOrDie
        val ln = context.value.path("lastName").flatMap(_.asString).getOrDie
        Remote.dynamicValue(fn ++ Remote(" ") ++ ln)
      },
      "comments" -> { context =>
        endpoints.userComments(Remote.record("email" -> context.value.path("email").getOrDie))
      },
      "albums"   -> { context =>
        endpoints.userAlbums(Remote.record("userId" -> context.value.path("id").getOrDie))
      },
      "todos"    -> { context =>
        endpoints.UserTodos(Remote.record("userId" -> context.value.path("id").getOrDie))
      }
    ),
    "Post"  -> List("comments" -> { context =>
      endpoints.postComments(Remote.record("postId" -> context.value.path("id").getOrDie))
    }),
    "Album" -> List("photos" -> { context =>
      endpoints.AlbumPhotos(Remote.record("albumId" -> context.value.path("id").getOrDie))
    })
  )

  def spec = suite("OrchSpec")(test("test")(assertCompletes))
}
