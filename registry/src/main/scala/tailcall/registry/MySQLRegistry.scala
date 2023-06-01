package tailcall.registry

import io.getquill._
import io.getquill.context.ZioJdbc.QuillZioDataSourceExt
import io.getquill.context.qzio.ImplicitSyntax
import tailcall.registry.model.BlueprintSpec
import tailcall.runtime.model.{Blueprint, Digest}
import zio.Task

import java.sql.Timestamp
import java.util.Date
import javax.sql.DataSource

final case class MySQLRegistry(source: javax.sql.DataSource, ctx: MysqlZioJdbcContext[SnakeCase])
    extends SchemaRegistry {
  import BlueprintSpec._

  implicit private val dataSource: ImplicitSyntax.Implicit[DataSource] = ImplicitSyntax.Implicit(source)
  import ctx._

  override def add(blueprint: Blueprint): Task[Digest] = {
    val blueprintSpec = BlueprintSpec(
      digestHex = blueprint.digest.hex,
      digestAlg = blueprint.digest.alg,
      blueprint = blueprint,
      created = new Timestamp(new java.util.Date().getTime),
    )

    val sql = quote(query[BlueprintSpec].insertValue(lift(blueprintSpec)))
    ctx.run(sql).as(blueprint.digest).implicitDS
  }

  override def drop(digest: Digest): Task[Boolean] = {
    val sql = quote(filterByDigest(digest).update(_.dropped -> lift(Option(new Timestamp(new Date().getTime)))))
    ctx.run(sql).map(_ > 0).implicitDS
  }

  override def get(digest: Digest): Task[Option[Blueprint]] = {
    val sql = quote(filterByDigest(digest).map(_.blueprint))
    ctx.run(sql).map(_.headOption).implicitDS
  }

  override def list(index: Int, max: Int): Task[List[Blueprint]] = {
    val sql = quote(query[BlueprintSpec].drop(lift(index)).take(lift(max)).map(_.blueprint))
    ctx.run(sql).implicitDS
  }

  private def filterByDigest(digest: Digest): Quoted[EntityQuery[BlueprintSpec]] =
    quote(query[BlueprintSpec].filter(b => b.digestHex == lift(digest.hex) && b.digestAlg == lift(digest.alg)))
}
