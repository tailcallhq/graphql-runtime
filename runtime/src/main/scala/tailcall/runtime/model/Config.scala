package tailcall.runtime.model

import tailcall.runtime.JsonT
import tailcall.runtime.http.Method
import tailcall.runtime.model.Config._
import tailcall.runtime.service.ConfigFileIO
import tailcall.runtime.transcoder.Transcoder
import zio.ZIO
import zio.json._

import java.io.File

final case class Config(version: Int = 0, server: Server = Server(), graphQL: GraphQL = GraphQL()) {
  self =>
  def ++(other: Config): Config = self.mergeRight(other)

  def mergeRight(other: Config): Config = {
    Config(
      version = other.version,
      server = self.server.mergeRight(other.server),
      graphQL = self.graphQL.mergeRight(other.graphQL),
    )
  }

  def compress: Config = self.copy(graphQL = self.graphQL.compress)

  def toBlueprint: Blueprint = Transcoder.toBlueprint(self).get

  def withMutation(mutation: String): Config = self.copy(graphQL = self.graphQL.withMutation(mutation))

  def withQuery(query: String): Config = self.copy(graphQL = self.graphQL.withQuery(query))

  def withRootSchema(
    query: Option[String] = graphQL.schema.query,
    mutation: Option[String] = graphQL.schema.mutation,
  ): Config = self.copy(graphQL = self.graphQL.copy(schema = RootSchema(query, mutation)))

  def withType(input: (String, Type)*): Config = {
    input.foldLeft(self) { case (config, (name, typeInfo)) =>
      config.copy(graphQL = config.graphQL.withType(name, typeInfo))
    }
  }
}

object Config {
  def empty: Config = Config()

  def default: Config = Config.empty.withQuery("Query")

  def fromFile(file: File): ZIO[ConfigFileIO, Throwable, Config] = ConfigFileIO.readFile(file)

  final case class RootSchema(query: Option[String] = None, mutation: Option[String] = None)

  final case class Type(doc: Option[String] = None, fields: Map[String, Field] = Map.empty) {
    self =>
    def ++(other: Type): Type = self.mergeRight(other)

    def mergeRight(other: Type): Type =
      self.copy(doc = other.doc.orElse(self.doc), fields = self.fields ++ other.fields)

    def argTypes: List[String] = fields.values.toList.flatMap(_.args.toList.flatMap(_.toList)).map(_._2.typeOf)

    def returnTypes: List[String] = fields.values.toList.map(_.typeOf)

    def compress: Type = self.copy(fields = self.fields.map { case (k, v) => k -> v.compress })

    def withDoc(doc: String): Type = self.copy(doc = Option(doc))

    def withFields(input: (String, Field)*): Type =
      input.foldLeft(self) { case (self, (name, field)) => self.withField(name, field) }

    def withField(name: String, field: Field): Type = self.copy(fields = self.fields + (name -> field))
  }

  object Type {
    def apply(fields: (String, Field)*): Type = Type(fields = fields.toMap)
    def empty: Type                           = Type()
  }

  final case class GraphQL(schema: RootSchema = RootSchema(), types: Map[String, Type] = Map.empty) {
    self =>
    def compress: GraphQL = self.copy(types = self.types.map { case (k, t) => (k, t.compress) })

    def mergeRight(other: GraphQL): GraphQL = {
      other.types.foldLeft(self) { case (config, (name, typeInfo)) => config.withType(name, typeInfo) }.copy(schema =
        RootSchema(
          query = other.schema.query.orElse(self.schema.query),
          mutation = other.schema.mutation.orElse(self.schema.mutation),
        )
      )
    }

    def withType(name: String, typeInfo: Type): GraphQL = {
      self.copy(types = self.types.get(name) match {
        case Some(typeInfo0) => self.types + (name -> (typeInfo0 mergeRight typeInfo))
        case None            => self.types + (name -> typeInfo)
      })
    }

    def withMutation(name: String): GraphQL = copy(schema = schema.copy(mutation = Option(name)))

    def withQuery(name: String): GraphQL = copy(schema = schema.copy(query = Option(name)))

    def withSchema(query: Option[String], mutation: Option[String]): GraphQL =
      copy(schema = RootSchema(query, mutation))
  }

  // TODO: Field and Argument can be merged
  final case class Field(
    @jsonField("type") typeOf: String,

    // TODO: rename to `list`
    @jsonField("isList") list: Option[Boolean] = None,

    // TODO: rename to `required`
    @jsonField("isRequired") required: Option[Boolean] = None,
    steps: Option[List[Step]] = None,
    args: Option[Map[String, Arg]] = None,
    doc: Option[String] = None,
    modify: Option[ModifyField] = None,
  ) {
    self =>

    def apply(args: (String, Arg)*): Field = copy(args = Option(args.toMap))

    def asList: Field = copy(list = Option(true))

    def asRequired: Field = copy(required = Option(true))

    def compress: Field = {
      val isList = self.list match {
        case Some(true) => Some(true)
        case _          => None
      }

      val isRequired = self.required match {
        case Some(true) => Some(true)
        case _          => None
      }

      val steps = self.steps match {
        case Some(steps) if steps.nonEmpty =>
          Option(steps.map {
            case step @ Step.Http(_, _, _, _) =>
              val noOutputHttp = step.withOutput(None).withInput(None)
              if (step.method contains Method.GET) noOutputHttp.copy(method = None) else noOutputHttp
            case step                         => step
          })
        case _                             => None
      }

      val args = self.args match {
        case Some(args) if args.nonEmpty => Some(args.map { case (k, v) => (k, v.compress) })
        case _                           => None
      }

      val update = self.modify match {
        case Some(value) if value.nonEmpty => Some(value)
        case _                             => None
      }

      self.copy(list = isList, required = isRequired, steps = steps, args = args, modify = update)
    }

    def isList: Boolean = list.getOrElse(false)

    def isRequired: Boolean = required.getOrElse(false)

    def withArguments(args: (String, Arg)*): Field = withArguments(args.toMap)

    def withArguments(args: Map[String, Arg]): Field = copy(args = Option(args))

    def withDoc(doc: String): Field = copy(doc = Option(doc))

    def withUpdate(update: ModifyField): Field =
      copy(modify = self.modify match {
        case Some(value) => Some(value mergeRight update)
        case None        => Some(update)
      })

    def withName(name: String): Field = withUpdate(ModifyField.empty.withName(name))

    def withSteps(steps: Step*): Field = copy(steps = Option(steps.toList))

    def resolveWith[A: JsonEncoder](a: A): Field = withSteps(Step.transform(JsonT.const(a.toJsonAST.toOption.get)))
  }

  object Field {
    def apply(str: String, operations: Step*): Field =
      Field(typeOf = str, steps = if (operations.isEmpty) None else Option(operations.toList))

    def bool: Field = Field(typeOf = "Boolean")

    def int: Field = Field(typeOf = "Int")

    def ofType(name: String): Field = Field(typeOf = name)

    def string: Field = Field(typeOf = "String")
  }

  final case class Arg(
    @jsonField("type") typeOf: String,

    // TODO: rename to `list`
    @jsonField("isList") list: Option[Boolean] = None,

    // TODO: rename to `required`
    @jsonField("isRequired") required: Option[Boolean] = None,
    doc: Option[String] = None,
    modify: Option[ModifyField] = None,
  ) {
    self =>
    def asList: Arg = self.copy(list = Option(true))

    def asRequired: Arg = self.copy(required = Option(true))

    def compress: Arg = {
      val isList = self.list match {
        case Some(true) => Some(true)
        case _          => None
      }

      val isRequired = self.required match {
        case Some(true) => Some(true)
        case _          => None
      }

      val update = self.modify match {
        case Some(value) if value.nonEmpty => Some(value)
        case _                             => None
      }

      self.copy(list = isList, required = isRequired, modify = update)
    }

    def isList: Boolean = list.getOrElse(false)

    def isRequired: Boolean = required.getOrElse(false)

    def withDoc(doc: String): Arg = copy(doc = Option(doc))

    def withUpdate(update: ModifyField): Arg =
      copy(modify = self.modify match {
        case Some(value) => Some(value mergeRight update)
        case None        => Some(update)
      })

    def withName(name: String): Arg = withUpdate(ModifyField.empty.withName(name))
  }

  object Arg {
    val string: Arg               = Arg("String")
    val int: Arg                  = Arg("Int")
    val bool: Arg                 = Arg("Boolean")
    def ofType(name: String): Arg = Arg(name)
  }

  implicit lazy val typeInfoCodec: JsonCodec[Type]               = DeriveJsonCodec.gen[Type]
  implicit lazy val inputTypeCodec: JsonCodec[Arg]               = DeriveJsonCodec.gen[Arg]
  implicit lazy val fieldAnnotationCodec: JsonCodec[ModifyField] = DeriveJsonCodec.gen[ModifyField]
  implicit lazy val fieldDefinitionCodec: JsonCodec[Field]       = DeriveJsonCodec.gen[Field]
  implicit lazy val schemaDefinitionCodec: JsonCodec[RootSchema] = DeriveJsonCodec.gen[RootSchema]
  implicit lazy val graphQLCodec: JsonCodec[GraphQL]             = DeriveJsonCodec.gen[GraphQL]
  implicit lazy val jsonCodec: JsonCodec[Config]                 = DeriveJsonCodec.gen[Config]
}
