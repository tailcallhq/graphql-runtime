package tailcall.runtime.model

import tailcall.runtime.DirectiveCodec
import zio.json.{DeriveJsonCodec, JsonCodec, jsonHint}

import java.net.{URI, URL}

@jsonHint("server")
final case class Server(
  baseURL: Option[URL] = None,
  timeout: Option[Int] = None,
  vars: Option[Map[String, String]] = None,
) {
  self =>
  def isEmpty: Boolean                  = baseURL.isEmpty && timeout.isEmpty && vars.isEmpty
  def mergeRight(other: Server): Server = {
    val vars = self.vars.flatMap(vars => other.vars.map(vars ++ _)).orElse(other.vars)
    Server(baseURL = other.baseURL.orElse(self.baseURL), timeout = other.timeout, vars = vars)
  }

  def compress: Server = {
    val vars = if (self.vars.exists(_.isEmpty)) None else self.vars
    self.copy(vars = vars)
  }
}

object Server {
  implicit val urlCodec: JsonCodec[URL]          = JsonCodec[String].transformOrFail[URL](
    string =>
      try Right(URI.create(string).toURL)
      catch { case _: Throwable => Left(s"Malformed url: ${string}") },
    _.toString,
  )
  implicit val json: JsonCodec[Server]           = DeriveJsonCodec.gen[Server]
  implicit val directive: DirectiveCodec[Server] = DirectiveCodec.fromJsonCodec("server", json)
}
