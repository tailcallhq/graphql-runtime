package tailcall.runtime.model

import tailcall.runtime.http.Method
import tailcall.runtime.{DirectiveCodec, JsonT}
import zio.json._
import zio.json.ast.Json

sealed trait Step

object Step {
  def objPath(spec: (String, List[String])*): Step = Transform(JsonT.objPath(spec: _*))
  def constant(a: Json): Step                      = Transform(JsonT.Constant(a))
  def transform(jsonT: JsonT): Step                = Transform(jsonT)

  @jsonHint("http")
  final case class Http(
    path: Path,
    method: Option[Method] = None,
    input: Option[TSchema] = None,
    output: Option[TSchema] = None,
  ) extends Step {
    def withInput(input: Option[TSchema]): Http   = copy(input = input)
    def withMethod(method: Method): Http          = copy(method = Option(method))
    def withOutput(output: Option[TSchema]): Http = copy(output = output)
  }

  @jsonHint("transform")
  final case class Transform(transformation: JsonT) extends Step
  object Transform {
    implicit val jsonCodec: JsonCodec[Transform] = JsonCodec[JsonT].transform(Transform(_), _.transformation)
  }

  object Http {
    private val jsonCodec: JsonCodec[Http] = DeriveJsonCodec.gen[Http]

    def fromEndpoint(endpoint: Endpoint): Http   =
      Http(path = endpoint.path, method = Option(endpoint.method), input = endpoint.input, output = endpoint.output)
    implicit val directive: DirectiveCodec[Http] = DirectiveCodec.fromJsonCodec("http", jsonCodec)
  }

  implicit lazy val jsonCodec: JsonCodec[Step] = DeriveJsonCodec.gen[Step]

  // TODO: this should be auto-generated
  implicit lazy val directive: DirectiveCodec[List[Step]] = DirectiveCodec.fromJsonListCodec("steps", jsonCodec)
}
