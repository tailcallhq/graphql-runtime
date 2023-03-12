package tailcall.runtime.dsl.scala

import tailcall.runtime.ast.Blueprint
import tailcall.runtime.dsl.scala.Orc.{Field, FieldSet, Input, LabelledField, Output}
import tailcall.runtime.remote.Remote
import zio.Task
import zio.schema.{DeriveSchema, DynamicValue, Schema}

/**
 * A scala DSL to create an orchestration specification.
 */
final case class Orc(
  query: Option[String] = None,
  mutation: Option[String] = None,
  subscription: Option[String] = None,
  types: List[Orc.Obj] = Nil
) {
  self =>
  def toBlueprint: Task[Blueprint] = OrcBlueprint.toBlueprint(self).mapError(new RuntimeException(_))

  def withQuery(name: String): Orc = self.copy(query = Option(name))

  def withInput(spec: (String, List[(String, Field[Input])])*): Orc = withTypes(spec.toList)(FieldSet.InputSet(_))

  def withOutput(spec: (String, List[(String, Field[Output])])*): Orc = withTypes(spec.toList)(FieldSet.OutputSet(_))

  def withTypes[A](spec: List[(String, List[(String, Field[A])])])(f: List[LabelledField[A]] => FieldSet): Orc =
    self.copy(types = self.types ++ spec.map { case (name, fields) =>
      Orc.Obj(name, f(fields.map { case (name, field) => LabelledField(name, field) }))
    })
}

object Orc {
  val empty: Orc = Orc(Option("Query"), Option("Mutation"), None, Nil)

  def input(spec: (String, List[(String, Field[Input])])*): Orc = Orc.empty.withInput(spec: _*)

  def output(spec: (String, List[(String, Field[Output])])*): Orc = Orc.empty.withOutput(spec: _*)

  def apply(spec: (String, List[(String, Field[Output])])*): Orc = Orc.empty.withOutput(spec: _*)

  sealed trait Dir[A]
  case object In  extends Dir[Input]
  case object Out extends Dir[Output]

  final case class LabelledField[A](name: String, field: Field[A])
  final case class Obj(name: String, fields: FieldSet = FieldSet.Empty) {
    def withFields(fields: LabelledField[Output]*): Obj = copy(fields = FieldSet.OutputSet(fields.toList))
    def withInputs(fields: LabelledField[Input]*): Obj  = copy(fields = FieldSet.InputSet(fields.toList))
    def withName(name: String): Obj                     = copy(name = name)
  }

  final case class Input(defaultValue: Option[DynamicValue])
  final case class Output(arguments: List[LabelledField[Input]] = Nil, resolve: Resolver)
  final case class Field[A](ofType: Option[Type], definition: A) {
    self =>
    def to(name: String): Field[A] = copy(ofType = Option(Type.NamedType(name)))

    def asList: Field[A] = copy(ofType = ofType.map(Type.ListType(_)))

    def asRequired: Field[A] = copy(ofType = ofType.map(Type.NonNull(_)))

    def resolveWith[T](t: T)(implicit s: Schema[T], ev: A <:< Output): Field[Output] =
      copy(definition = definition.copy(resolve = Resolver.fromFunction(_ => Remote(DynamicValue(t)))))

    def resolveWithFunction(f: Remote[DynamicValue] => Remote[DynamicValue])(implicit ev: A <:< Output): Field[Output] =
      copy(definition = definition.copy(resolve = Resolver.fromFunction(f)))

    def withDefault[T](t: T)(implicit s: Schema[T], ev: A <:< Input): Field[Input] =
      copy(definition = definition.copy(defaultValue = Option(DynamicValue(t))))

    def withArgument(fields: (String, Field[Input])*)(implicit ev: A <:< Output): Field[Output] =
      copy(definition = definition.copy(arguments = fields.toList.map(f => LabelledField(f._1, f._2))))
  }

  sealed trait FieldSet
  object FieldSet {
    final case class InputSet(fields: List[LabelledField[Input]])   extends FieldSet
    final case class OutputSet(fields: List[LabelledField[Output]]) extends FieldSet
    case object Empty                                               extends FieldSet
  }

  sealed trait Resolver
  object Resolver {
    def fromFunction(f: Remote[DynamicValue] => Remote[DynamicValue]): Resolver = FromFunction(f)
    def empty: Resolver                                                         = Empty
    final case class FromFunction(f: Remote[DynamicValue] => Remote[DynamicValue]) extends Resolver
    case object Empty                                                              extends Resolver
  }

  object Field {
    def input: Field[Input]   = Field(None, Input(None))
    def output: Field[Output] = Field(None, Output(Nil, Resolver.empty))
  }

  sealed trait Type
  object Type {
    final case class NonNull(ofType: Type)   extends Type
    final case class NamedType(name: String) extends Type
    final case class ListType(ofType: Type)  extends Type
  }

  implicit val schema: Schema[Orc] = DeriveSchema.gen[Orc]
}
