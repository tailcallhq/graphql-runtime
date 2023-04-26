package tailcall.runtime.lambda.operations

import tailcall.runtime.JsonT
import tailcall.runtime.lambda._
import zio.schema.{DynamicValue, Schema}

trait DynamicValueOps {
  implicit final class LambdaDynamicValueOps[A](private val self: A ~> DynamicValue) {
    def toTyped[B](implicit schema: Schema[B]): A ~> Option[B] = self >>> Lambda.dynamic.toTyped[B]

    def path(name: String*): A ~> Option[DynamicValue] = self >>> Lambda.dynamic.path(name: _*)

    def pathSeq(name: String*): A ~> Option[DynamicValue] = self >>> Lambda.dynamic.pathSeq(name: _*)

    def toTypedPath[B](name: String*)(implicit schema: Schema[B]): A ~> Option[B] =
      self.path(name: _*).flatMap(_.toTyped[B])

    def transform(jsonT: JsonT): A ~> DynamicValue = self >>> Lambda.dynamic.jsonTransform(jsonT)
  }
}
