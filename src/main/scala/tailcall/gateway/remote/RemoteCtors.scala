package tailcall.gateway.remote

import tailcall.gateway.ast.Endpoint
import zio.Chunk
import zio.schema.{DynamicValue, Schema}

trait RemoteCtors {
  def apply[A](a: A)(implicit ctor: Constructor[A]): Remote[A] =
    Remote.unsafe.attempt(DynamicEval.cons(ctor.schema.toDynamic(a), ctor))

  def fromFunction[A, B](ab: Remote[A] => Remote[B]): Remote[A => B] =
    Remote
      .unsafe
      .attempt {
        val id = DynamicEval.binding
        DynamicEval.bind(id, ab(Remote.unsafe.attempt[A](id)).compile)
      }

  def fromSeq[A](a: Seq[Remote[A]]): Remote[Seq[A]] =
    Remote.unsafe.attempt(DynamicEval.seq(a.map(_.compile)))

  def fromMap[A, B](a: Map[Remote[A], Remote[B]]): Remote[Map[A, B]] =
    Remote
      .unsafe
      .attempt(DynamicEval.map(a.map { case (k, v) => k.compile -> v.compile }))

  def fromEither[E, A](a: Either[Remote[E], Remote[A]]): Remote[Either[E, A]] =
    Remote
      .unsafe
      .attempt(DynamicEval.either(a match {
        case Left(value)  => Left(value.compile)
        case Right(value) => Right(value.compile)
      }))

  def fromOption[A](a: Option[Remote[A]]): Remote[Option[A]] =
    Remote.unsafe.attempt(DynamicEval.option(a.map(_.compile)))

  def none[B]: Remote[Option[B]] = Remote.unsafe.attempt(DynamicEval.none)

  def fromEndpoint(endpoint: Endpoint): Remote[DynamicValue => DynamicValue] =
    Remote.fromFunction[DynamicValue, DynamicValue](input =>
      Remote.unsafe.attempt(DynamicEval.endpoint(endpoint, input.compile))
    )

  def dynamicValue[A](a: A)(implicit schema: Schema[A]): Remote[DynamicValue] =
    Remote(Schema.toDynamic(a))

  def record(fields: (String, Remote[DynamicValue])*): Remote[DynamicValue] =
    Remote
      .unsafe
      .attempt(DynamicEval.record(fields.map { case (k, v) => k -> v.compile }))

  def die(msg: Remote[String]): Remote[Nothing] =
    Remote.unsafe.attempt(DynamicEval.die(msg.compile))

  def die(msg: String): Remote[Nothing] = die(Remote(msg))

  def fromTuple[A1, A2](t: (Remote[A1], Remote[A2])): Remote[(A1, A2)] =
    Remote.unsafe.attempt(DynamicEval.tuple(Chunk(t._1.compile, t._2.compile)))

  def fromTuple[A1, A2, A3](
    t: (Remote[A1], Remote[A2], Remote[A3])
  ): Remote[(A1, A2, A3)] =
    Remote
      .unsafe
      .attempt(DynamicEval.tuple(
        Chunk(t._1.compile, t._2.compile, t._3.compile)
      ))

  def fromTuple[A1, A2, A3, A4](
    t: (Remote[A1], Remote[A2], Remote[A3], Remote[A4])
  ): Remote[(A1, A2, A3, A4)] =
    Remote
      .unsafe
      .attempt(DynamicEval.tuple(
        Chunk(t._1.compile, t._2.compile, t._3.compile, t._4.compile)
      ))

  def batch[A, B, C](
    from: Remote[Seq[A]],
    to: Remote[Seq[B]] => Remote[Seq[C]],
    ab: Remote[A] => Remote[B],
    ba: Remote[B] => Remote[A],
    cb: Remote[C] => Remote[B]
  ) = {
    val v = from.map(ab(_))
    v.map(i =>
      fromTuple(
        ba(i),
        to(v)
          .map(c => fromTuple((cb(c), c)))
          .groupBy(_._1)
          .get(i)
          .flatMap(x => x.map(_._2).head) // Todo: Add flatten in Option
      )
    )
  }
}
