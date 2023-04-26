package tailcall.runtime.service

import tailcall.runtime.internal.DynamicValueUtil
import tailcall.runtime.lambda._
import tailcall.runtime.transcoder.Transcoder
import zio._
import zio.json.DecoderOps
import zio.json.ast.Json
import zio.schema.{DynamicValue, Schema}

import java.nio.charset.StandardCharsets

trait EvaluationRuntime {
  final def evaluate[A, B](lambda: A ~> B): LExit[HttpContext, Throwable, A, B] =
    evaluate(lambda, EvaluationContext.make)

  final def evaluate[A, B](lambda: A ~> B, ctx: EvaluationContext): LExit[HttpContext, Throwable, A, B] =
    evaluate(lambda.compile(CompilationContext.initial), ctx).asInstanceOf[LExit[Any, Throwable, A, B]]

  def evaluate(dynamicEval: Expression, ctx: EvaluationContext): LExit[HttpContext, Throwable, Any, Any]

  final def evaluateAs[A](eval: Expression, ctx: EvaluationContext): LExit[HttpContext, Throwable, Any, A] =
    evaluate(eval, ctx).flatMap(a => LExit.attempt(a.asInstanceOf[A]))
}

object EvaluationRuntime {
  import Expression._

  def evaluate[A, B](ab: A ~> B): LExit[EvaluationRuntime with HttpContext, Throwable, A, B] =
    LExit.fromZIO(ZIO.service[EvaluationRuntime]).flatMap(_.evaluate(ab))

  def default: ZLayer[Any, Nothing, EvaluationRuntime] = ZLayer.succeed(new Live())

  final class Live extends EvaluationRuntime {
    override def evaluate(plan: Expression, ctx: EvaluationContext): LExit[HttpContext, Throwable, Any, Any] = {
      plan match {
        case Literal(value, meta)              => value.toTypedValue(meta.toSchema.asInstanceOf[Schema[Any]]) match {
            case Left(cause)  => LExit
                .fail(new RuntimeException(s"DynamicValue $value could not be decoded using ${schema}: ${cause}"))
            case Right(value) => LExit.succeed(value)
          }
        case EqualTo(left, right, tag)         => for {
            leftValue  <- evaluate(left, ctx)
            rightValue <- evaluate(right, ctx)
          } yield tag.toEquatable.equal(leftValue, rightValue)
        case Math(operation, tag)              => operation match {
            case Math.Binary(operation, left, right) =>
              for {
                leftValue  <- evaluate(left, ctx)
                rightValue <- evaluate(right, ctx)
              } yield operation match {
                case Math.Binary.Add              => tag.numeric.add(leftValue, rightValue)
                case Math.Binary.Multiply         => tag.numeric.multiply(leftValue, rightValue)
                case Math.Binary.Divide           => tag.numeric.divide(leftValue, rightValue)
                case Math.Binary.Modulo           => tag.numeric.modulo(leftValue, rightValue)
                case Math.Binary.GreaterThan      => tag.numeric.greaterThan(leftValue, rightValue)
                case Math.Binary.GreaterThanEqual => tag.numeric.greaterThanEqual(leftValue, rightValue)
              }
            case Math.Unary(operation, value)        =>
              for { value <- evaluate(value, ctx) } yield operation match { case Math.Unary.Negate => tag.numeric.negate(value) }
          }
        case Logical(operation)                => operation match {
            case Logical.Binary(operation, left, right) =>
              for {
                leftValue  <- evaluateAs[Boolean](left, ctx)
                rightValue <- evaluateAs[Boolean](right, ctx)
              } yield operation match {
                case Logical.Binary.And => leftValue && rightValue
                case Logical.Binary.Or  => leftValue || rightValue
              }
            case Logical.Unary(value, operation)        => evaluateAs[Boolean](value, ctx).flatMap { a =>
                operation match {
                  case Logical.Unary.Not                      => LExit.succeed(!a)
                  case Logical.Unary.Diverge(isTrue, isFalse) =>
                    if (a) evaluate(isTrue, ctx) else evaluate(isFalse, ctx)
                }
              }
          }
        case Identity                          => LExit.input
        case Pipe(left, right)                 => evaluate(left, ctx) >>> evaluate(right, ctx)
        case FunctionDef(binding, body, input) => for {
            i <- evaluate(input, ctx)
            r <- evaluate(body, ctx.set(binding, i))
          } yield r
        case Lookup(binding)                   => LExit.fromZIO {
            val ref = ctx.get(binding)
            for {
              res <- ref match {
                case Some(value) => ZIO.succeed(value)
                case None        => ZIO.fail(new RuntimeException(s"Binding not found: ${binding}"))
              }
            } yield res
          }

        case Immediate(eval0)   => for {
            eval1 <- evaluateAs[Expression](eval0, ctx)
            eval2 <- evaluate(eval1, ctx)
          } yield eval2
        case Defer(value)       => LExit.succeed(value)
        case Dynamic(operation) => LExit.input[Any].map(input =>
            operation match {
              case Dynamic.Typed(meta)     => DynamicValueUtil.toTyped(input.asInstanceOf[DynamicValue])(meta.toSchema)
              case Dynamic.ToDynamic(meta) => meta.toSchema.asInstanceOf[Schema[Any]].toDynamic(input)
              case Dynamic.Path(path, nestSeq)      => DynamicValueUtil
                  .getPath(input.asInstanceOf[DynamicValue], path, nestSeq)
              case Dynamic.JsonTransform(transform) => transform.run(input.asInstanceOf[DynamicValue])
            }
          )
        case Dict(operation)    => operation match {
            case Dict.Get(key, map) => for {
                k <- evaluate(key, ctx)
                m <- evaluateAs[Map[Any, Any]](map, ctx)
              } yield m.get(k)

            case Dict.Put(key, value, map) => for {
                k <- evaluate(key, ctx)
                v <- evaluate(value, ctx)
                m <- evaluateAs[Map[Any, Any]](map, ctx)
              } yield m.updated(k, v)

            case Dict.ToPair => for { map <- LExit.input[Any].map(_.asInstanceOf[Map[_, _]]) } yield map.toList
          }
        case Opt(operation)     => operation match {
            case Opt.IsSome                  => LExit.input.map(_.asInstanceOf[Option[_]].isDefined)
            case Opt.IsNone                  => LExit.input.map(_.asInstanceOf[Option[_]].isEmpty)
            case Opt.Fold(value, none, some) => for {
                opt <- evaluateAs[Option[_]](value, ctx)
                res <- opt match {
                  case Some(value) => evaluate(some, ctx).provideInput(value)
                  case None        => evaluate(none, ctx)
                }
              } yield res
            case Opt.Apply(value)            => value match {
                case None        => LExit.succeed(None)
                case Some(value) => for { any <- evaluate(value, ctx) } yield Option(any)
              }
          }
        case Unsafe(operation)  => operation match {
            case Unsafe.Debug(prefix)          => for {
                input <- LExit.input[Any]
                _     <- LExit.fromZIO(Console.printLine(s"${prefix}: $input"))
              } yield input
            case Unsafe.EndpointCall(endpoint) => for {
                input <- LExit.input[Any]
                out   <- LExit.fromZIO {
                  for {
                    chunk <- DataLoader.load(endpoint.evaluate(input.asInstanceOf[DynamicValue]))
                    json  <- ZIO.fromEither(new String(chunk.toArray, StandardCharsets.UTF_8).fromJson[Json])
                      .mapError(ValidationError.DecodingError("String", "JsonAST", _))
                    any   <- Transcoder.toDynamicValue(json).toZIO.mapError(_.mkString(", "))
                      .mapError(new RuntimeException(_))
                  } yield any
                }
              } yield out
          }
      }
    }
  }
}
