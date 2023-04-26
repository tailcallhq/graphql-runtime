package tailcall.runtime

import tailcall.runtime.internal.DynamicValueUtil._
import tailcall.runtime.internal.{CalibanGen, JsonGen, PrimitiveGen, TValid}
import tailcall.runtime.transcoder.Transcoder
import zio.durationInt
import zio.schema.DynamicValue
import zio.test.TestAspect.timeout
import zio.test._

object DynamicValueUtilSpec extends ZIOSpecDefault {
  override def spec =
    suite("DynamicValueUtilSpec")(
      suite("asString")(
        test("valid") {
          val dynamics: Gen[Any, (DynamicValue, String)] = Gen.oneOf(PrimitiveGen.genPrimitive.map { primitive =>
            primitive.toDynamicValue -> primitive.value.toString
          })

          checkAll(dynamics) { case (dynamic, expected) => assertTrue(asString(dynamic) == Some(expected)) }
        },
        test("invalid") {
          val dynamics: Gen[Any, DynamicValue] = Gen.fromIterable(Seq(DynamicValue(List(42)), DynamicValue(Option(1))))
          checkAll(dynamics)(dynamic => assertTrue(asString(dynamic) == None))
        },
      ),
      suite("toTyped")(
        test("valid") {
          val gen = Gen.fromIterable(Seq(
            toTyped[String](DynamicValue("Hello World!")) -> Some("Hello World!"),
            toTyped[Int](DynamicValue(42))                -> Some(42),
          ))

          checkAll(gen) { case (dynamicValue, expected) => assertTrue(dynamicValue == expected) }
        },
        test("invalid") {
          val gen = Gen.fromIterable(Seq(toTyped[Int](DynamicValue("Hello World!")), toTyped[String](DynamicValue(42))))

          checkAll(gen)(dynamicValue => assertTrue(dynamicValue == None))
        },
      ),
      suite("getPath")(
        test("valid") {
          val gen = Gen.fromIterable(Seq(
            DynamicValue(Map("a" -> 1))                         -> List("a")           -> 1,
            DynamicValue(Map("a" -> Map("b" -> 1)))             -> List("a", "b")      -> 1,
            DynamicValue(Map("a" -> Option(Map("b" -> 1))))     -> List("a", "b")      -> 1,
            DynamicValue(Map("a" -> Map("b" -> Map("c" -> 1)))) -> List("a", "b", "c") -> 1,
            DynamicValue(Map("a" -> List(Map("b" -> 1))))       -> List("a", "0", "b") -> 1,
            record("a" -> DynamicValue(1))                      -> List("a")           -> 1,
          ))

          checkAll(gen) { case dynamic -> path -> expected =>
            assertTrue(getPath(dynamic, path) == Some(DynamicValue(expected)))
          }
        },
        test("invalid") {
          val gen = Gen.fromIterable(Seq(
            DynamicValue(Map("a" -> 1))                         -> List("b"),
            DynamicValue(Map("a" -> Map("b" -> 1)))             -> List("b", "b"),
            DynamicValue(Map("a" -> Option(Map("b" -> 1))))     -> List("a", "c"),
            DynamicValue(Map("a" -> Map("b" -> Map("c" -> 1)))) -> List("a", "c", "e"),
            DynamicValue(Map("a" -> List(Map("b" -> 1))))       -> List("a", "1", "b"),
            record("a" -> DynamicValue(1))                      -> List("d"),
          ))

          checkAll(gen) { case dynamic -> path => assertTrue(getPath(dynamic, path) == None) }
        },
        test("lists") {
          val gen = Gen.fromIterable(Seq(
            DynamicValue(Map("a" -> List(Map("b" -> List(Map("c" -> 1)))))) -> List("a", "b", "c") -> List(List(1))
          ))

          checkAll(gen) { case dynamic -> path -> expected =>
            assertTrue(getPath(dynamic, path, true) == Some(DynamicValue(expected)))
          }
        },
      ),
      test("fromResponseValue >>> toResponseValue == identity") {
        check(CalibanGen.genResponseValue) { responseValue =>
          val actual   = Transcoder.toDynamicValue(responseValue).flatMap(Transcoder.toResponseValue(_))
          val expected = TValid.succeed(responseValue)
          assertTrue(actual == expected)
        }
      },
      test("fromInputValue >>> toInputValue == identity") {
        check(CalibanGen.genInputValue) { inputValue =>
          val actual   = Transcoder.toDynamicValue(inputValue).flatMap(Transcoder.toInputValue(_))
          val expected = TValid.succeed(inputValue)
          assertTrue(actual == expected)
        }
      },
      test("fromJson >>> toJson == identity")(check(JsonGen.genJson)(json => {
        val actual   = Transcoder.toDynamicValue(json).flatMap(Transcoder.toJson(_))
        val expected = TValid.succeed(json)
        assertTrue(actual == expected)
      })),
    ) @@ timeout(10.seconds)
}
