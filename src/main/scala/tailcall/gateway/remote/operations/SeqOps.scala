package tailcall.gateway.remote.operations

import tailcall.gateway.remote.{DynamicEval, Remote}

trait SeqOps {
  implicit final class RemoteSeqOps[A](val self: Remote[Seq[A]]) {
    def ++(other: Remote[Seq[A]]): Remote[Seq[A]] =
      Remote.unsafe.attempt(DynamicEval.concat(self.compile, other.compile))

    final def reverse: Remote[Seq[A]] =
      Remote.unsafe.attempt(DynamicEval.reverse(self.compile))

    final def filter(f: Remote[A] => Remote[Boolean]): Remote[Seq[A]] =
      Remote
        .unsafe
        .attempt(
          DynamicEval
            .filter(self.compile, Remote.fromFunction(f).compileAsFunction)
        )

    def find(f: Remote[A] => Remote[Boolean]): Remote[Option[A]] =
      filter(f).head

    final def flatMap[B](f: Remote[A] => Remote[Seq[B]]): Remote[Seq[B]] =
      Remote
        .unsafe
        .attempt(
          DynamicEval
            .flatMap(self.compile, Remote.fromFunction(f).compileAsFunction)
        )

    final def map[B](f: Remote[A] => Remote[B]): Remote[Seq[B]] =
      self.flatMap(a => Remote.fromSeq(Seq(f(a))))

    final def length: Remote[Int] =
      Remote.unsafe.attempt(DynamicEval.length(self.compile))

    final def indexOf(other: Remote[A]): Remote[Int] =
      Remote.unsafe.attempt(DynamicEval.indexOf(self.compile, other.compile))

    final def take(n: Int): Remote[Seq[A]] = slice(0, n)

    final def slice(from: Int, until: Int): Remote[Seq[A]] =
      Remote.unsafe.attempt(DynamicEval.slice(self.compile, from, until))

    final def head: Remote[Option[A]] =
      Remote.unsafe.attempt(DynamicEval.head(self.compile))

    final def groupBy[B](f: Remote[A] => Remote[B]): Remote[Map[B, Seq[A]]] =
      Remote
        .unsafe
        .attempt(
          DynamicEval
            .groupBy(self.compile, Remote.fromFunction(f).compileAsFunction)
        )
  }
}
