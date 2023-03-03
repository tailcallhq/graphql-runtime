package tailcall.gateway.service

import caliban.schema.Step
import tailcall.gateway.ast
import tailcall.gateway.ast.{Context, Document}
import tailcall.gateway.internal.DynamicValueUtil
import tailcall.gateway.remote.Remote
import zio.query.ZQuery
import zio.schema.DynamicValue
import zio.{ZIO, ZLayer}

import scala.collection.mutable

trait StepGenerator {
  def resolve(document: Document): Option[Step[Any]]
}

object StepGenerator {
  final case class Live(rtm: EvaluationRuntime) extends StepGenerator {
    private val stepRef: mutable.Map[String, Context => Step[Any]] = mutable.Map.empty

    def fromFieldDefinition(field: Document.FieldDefinition, ctx: Context): Step[Any] = {
      Step.FunctionStep { args =>
        val context = ctx.copy(args = args.view.mapValues(DynamicValueUtil.fromInputValue).toMap)
        field.resolver match {
          case Some(resolver) =>
            val step = for {
              value <- rtm.evaluate(resolver(Remote(DynamicValue(context))))
              step = fromType(field.ofType, context.copy(value = value))
            } yield step

            Step.QueryStep(ZQuery.fromZIO(step))
          case None           => fromType(field.ofType, context)
        }
      }
    }

    def fromType(tpe: ast.Document.Type, ctx: Context): Step[Any] =
      tpe match {
        case ast.Document.NamedType(name, _)  => stepRef
            .getOrElse(name, (ctx: Context) => Step.PureStep(DynamicValueUtil.toValue(ctx.value)))(ctx)
        case ast.Document.ListType(ofType, _) => ctx.value match {
            case DynamicValue.Sequence(values) => Step
                .ListStep(values.map(value => fromType(ofType, ctx.copy(value = value))).toList)
            case _                             => Step.ListStep(List(fromType(ofType, ctx)))
          }
      }

    def fromObjectDef(obj: Document.ObjectTypeDefinition, ctx: Context): Step[Any] = {
      Step.ObjectStep(obj.name, obj.fields.map(field => field.name -> fromFieldDefinition(field, ctx)).toMap)
    }

    override def resolve(document: Document): Option[Step[Any]] = {
      val rootContext = Context(DynamicValue(()))
      document.definition.collect { case obj @ Document.ObjectTypeDefinition(_, _) =>
        stepRef.put(obj.name, ctx => fromObjectDef(obj, ctx))
      }

      for {
        query <- document.definition.collectFirst { case Document.SchemaDefinition(query, _, _) => query }.flatten
        step  <- stepRef.get(query)
      } yield step(rootContext)
    }
  }

  def live: ZLayer[EvaluationRuntime, Nothing, StepGenerator] = {
    ZLayer(ZIO.service[EvaluationRuntime].map(rtm => Live(rtm)))
  }

  def resolve(document: Document): ZIO[StepGenerator, Nothing, Option[Step[Any]]] = ZIO.serviceWith(_.resolve(document))
}
