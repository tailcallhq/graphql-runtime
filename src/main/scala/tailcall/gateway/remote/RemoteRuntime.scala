package tailcall.gateway.remote

import tailcall.gateway.ast.Context
import tailcall.gateway.http.HttpClient
import tailcall.gateway.internal.ChunkUtil
import zio.schema.codec.JsonCodec
import zio.schema.{DynamicValue, Schema, TypeId}
import zio.{Task, ZIO, ZLayer}

import java.nio.charset.StandardCharsets
import scala.collection.immutable.ListMap

trait RemoteRuntime {
  final def evaluateAs[A](eval: DynamicEval): Task[A] =
    evaluate(eval).flatMap(any => ZIO.attempt(any.asInstanceOf[A]))

  final def evaluate[A](remote: Remote[A]): Task[A] =
    evaluateAs[A](remote.compile(CompilationContext.initial))

  def evaluate(eval: DynamicEval): Task[Any]
}

object RemoteRuntime {
  import DynamicEval._
  final class Default(val context: EvaluationContext) extends RemoteRuntime {
    def call[A](eval: DynamicEval, arg: Any): Task[A] = {
      val func = eval.asInstanceOf[FunctionDef]
      for {
        _      <- context.set(func.arg.id, arg)
        result <- evaluateAs[A](func.body)
        _      <- context.drop(func.arg.id)
      } yield result
    }
    def evaluate(eval: DynamicEval): Task[Any]        =
      eval match {
        case Literal(value, ctor) => ZIO
            .fromEither(value.toTypedValue(ctor.schema))
            .mapError(cause =>
              EvaluationError.TypeError(value, cause, ctor.schema)
            )

        case EqualTo(left, right, tag)   => for {
            leftValue  <- evaluate(left)
            rightValue <- evaluate(right)
          } yield tag.equal(leftValue, rightValue)
        case Math(operation, tag)        => operation match {
            case Math.Binary(left, right, operation) =>
              for {
                leftValue  <- evaluate(left)
                rightValue <- evaluate(right)
              } yield operation match {
                case Math.Binary.Add      => tag.add(leftValue, rightValue)
                case Math.Binary.Multiply => tag.multiply(leftValue, rightValue)
                case Math.Binary.Divide   => tag.divide(leftValue, rightValue)
                case Math.Binary.Modulo   => tag.modulo(leftValue, rightValue)
                case Math.Binary.GreaterThan =>
                  tag.greaterThan(leftValue, rightValue)
              }
            case Math.Unary(value, operation) => evaluate(value).map(evaluate =>
                operation match {
                  case Math.Unary.Negate => tag.negate(evaluate)
                }
              )
          }
        case Logical(operation)          => operation match {
            case Logical.Binary(left, right, operation) =>
              for {
                leftValue  <- evaluateAs[Boolean](left)
                rightValue <- evaluateAs[Boolean](right)
              } yield operation match {
                case Logical.Binary.And => leftValue && rightValue
                case Logical.Binary.Or  => leftValue || rightValue
              }
            case Logical.Unary(value, operation) => evaluateAs[Boolean](value)
                .flatMap { a =>
                  operation match {
                    case Logical.Unary.Not => ZIO.succeed(!a)
                    case Logical.Unary.Diverge(isTrue, isFalse) =>
                      if (a) evaluate(isTrue) else evaluate(isFalse)
                  }
                }
          }
        case StringOperations(operation) => operation match {
            case StringOperations.Concat(left, right) => for {
                leftValue  <- evaluateAs[String](left)
                rightValue <- evaluateAs[String](right)
              } yield leftValue ++ rightValue
          }
        case MapOperations(operation)    => operation match {
            case MapOperations.Get(map, key) => for {
                map <- evaluateAs[Map[Any, Any]](map)
                key <- evaluateAs[Any](key)
              } yield map.get(key)
            case MapOperations.Cons(values)  =>
              val result = ZIO.foreach(values) { case (key, value) =>
                evaluate(value).map(key -> _)
              }
              result.map(_.toMap)
          }
        case SeqOperations(operation)    => operation match {
            case SeqOperations.Concat(left, right)    => for {
                leftValue  <- evaluateAs[Seq[_]](left)
                rightValue <- evaluateAs[Seq[_]](right)
              } yield leftValue ++ rightValue
            case SeqOperations.IndexOf(seq, element)  => for {
                seq <- evaluateAs[Seq[_]](seq)
                e   <- evaluate(element)
              } yield seq.indexOf(e)
            case SeqOperations.Reverse(seq)           =>
              evaluateAs[Seq[_]](seq).map(_.reverse)
            case SeqOperations.Filter(seq, condition) => for {
                seq    <- evaluateAs[Seq[_]](seq)
                result <- ZIO.filter(seq)(any => call[Boolean](condition, any))
              } yield result

            case SeqOperations.FlatMap(seq, operation)   => for {
                seq    <- evaluateAs[Seq[Any]](seq)
                result <- ZIO.foreach(seq)(any => call[Seq[_]](operation, any))
              } yield result.flatten
            case SeqOperations.Length(seq)               =>
              evaluateAs[Seq[_]](seq).map(_.length)
            case SeqOperations.Slice(seq, from, to)      => for {
                seq    <- evaluateAs[Seq[_]](seq)
                result <- ZIO.succeed(seq.slice(from, to))
              } yield result
            case SeqOperations.Head(seq)                 =>
              evaluateAs[Seq[_]](seq).map(_.headOption)
            case SeqOperations.Sequence(value, _)        =>
              ZIO.foreach(value)(evaluate)
            case SeqOperations.GroupBy(seq, keyFunction) => for {
                seq <- evaluateAs[Seq[Any]](seq)
                map <- ZIO.foreach(seq)(any =>
                  call[Any](keyFunction, any).map(_ -> any)
                )
              } yield map.groupBy(_._1).map { case (k, v) => k -> v.map(_._2) }
          }
        case EitherOperations(operation) => operation match {
            case EitherOperations.Cons(value)              => value match {
                case Left(value)  => evaluate(value).map(Left(_))
                case Right(value) => evaluate(value).map(Right(_))
              }
            case EitherOperations.Fold(value, left, right) => for {
                either <- evaluateAs[Either[_, _]](value)
                result <- either match {
                  case Left(value)  => call[Any](left, value)
                  case Right(value) => call[Any](right, value)
                }
              } yield result
          }
        case FunctionCall(arg, f)        => evaluate(arg).flatMap(call(f, _))
        case Lookup(id)                  => context.get(id)
        case FunctionDef(_, _)           => ???
        case OptionOperations(operation) => operation match {
            case OptionOperations.Cons(option)            => option match {
                case Some(value) => evaluate(value).map(Some(_))
                case None        => ZIO.none
              }
            case OptionOperations.Fold(value, none, some) => for {
                option <- evaluateAs[Option[_]](value)
                result <- option match {
                  case Some(value) => call(some, value)
                  case None        => evaluate(none)
                }
              } yield result
          }

        case Die(message)   => evaluateAs[String](message)
            .flatMap(message => ZIO.fail(EvaluationError.Death(message)))
        case Record(fields) => for {
            f <- ZIO.foreach(fields)(field =>
              evaluateAs[DynamicValue](field._2).map(field._1 -> _)
            )
          } yield DynamicValue.Record(TypeId.Structural, ListMap.from(f))

        case TupleOperations(operations) => operations match {
            case TupleOperations.Cons(values)       => for {
                any <- ZIO.foreach(values)(evaluate)
                tup <- ChunkUtil.toTuple(any) match {
                  case null    =>
                    ZIO.fail(EvaluationError.InvalidTupleSize(any.length))
                  case product => ZIO.succeed(product)
                }
              } yield tup
            case TupleOperations.GetIndex(value, i) => for {
                f <- evaluateAs[Product](value)
              } yield f.productIterator.toSeq(i)
          }

        case ContextOperations(self, operation) => evaluateAs[Context](self)
            .map { ctx =>
              operation match {
                case ContextOperations.GetArg(name) => ctx.args.get(name)
                case ContextOperations.GetValue     => ctx.value
                case ContextOperations.GetParent    => ctx.parent
              }
            }

        case EndpointCall(endpoint, arg) => for {
            input <- evaluateAs[DynamicValue](arg)
            req = endpoint.evaluate(input).toHttpRequest
            array <- ZIO.async[Any, Nothing, Array[Byte]](cb =>
              HttpClient
                .make
                .request(req)((_, _, body) => cb(ZIO.succeed(body)))
            )
            outputSchema = endpoint.outputSchema.asInstanceOf[Schema[Any]]
            any <- ZIO
              .fromEither(
                JsonCodec
                  .jsonDecoder(outputSchema)
                  .decodeJson(new String(array, StandardCharsets.UTF_8))
                  .map(outputSchema.toDynamic)
              )
              .mapError(EvaluationError.DecodingError)
          } yield any

        case _: DynamicValueOperations => ???

        case Debug(self, prefix) => evaluate(self).debug(prefix)

        case Recurse(_) => ???

        case Flatten(eval) => evaluateAs[Remote[_]](eval).flatMap(evaluate(_))
      }
  }

  def live: ZLayer[EvaluationContext, Nothing, RemoteRuntime] =
    ZLayer.fromZIO(ZIO.service[EvaluationContext].map(ctx => new Default(ctx)))

  def evaluate[A](remote: Remote[A]) =
    ZIO.serviceWithZIO[RemoteRuntime](
      _.evaluateAs[A](remote.compile(CompilationContext.initial))
    )
}
