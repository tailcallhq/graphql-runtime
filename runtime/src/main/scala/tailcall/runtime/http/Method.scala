package tailcall.runtime.http

import zio.json.JsonCodec

sealed trait Method {
  def name: String = Method.encode(this)
}

object Method {
  case object GET    extends Method
  case object POST   extends Method
  case object PUT    extends Method
  case object DELETE extends Method

  def encode(method: Method): String =
    method match {
      case Method.GET    => "GET"
      case Method.POST   => "POST"
      case Method.PUT    => "PUT"
      case Method.DELETE => "DELETE"
    }

  def decode(method: String): Either[String, Method] =
    method match {
      case "GET"    => Right(Method.GET)
      case "POST"   => Right(Method.POST)
      case "PUT"    => Right(Method.PUT)
      case "DELETE" => Right(Method.DELETE)
      case name     => Left("Unknown method: " + name)
    }

  implicit lazy val methodCodec: JsonCodec[Method] = JsonCodec[String].transformOrFail(Method.decode, Method.encode)
}
