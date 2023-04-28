package tailcall.runtime

import tailcall.runtime.model.Mustache
import tailcall.runtime.model.Mustache.{MustacheExpression, prm, txt}
import zio.schema.DynamicValue
import zio.test.Assertion._
import zio.test._

object MustacheSpec extends ZIOSpecDefault {
  def spec =
    suite("Mustache")(
      test("syntax") {
        val input = List(
          "{{a}}"     -> MustacheExpression("a"),
          "{{a.b}}"   -> MustacheExpression("a", "b"),
          "{{a.b.c}}" -> MustacheExpression("a", "b", "c"),
        )

        checkAll(Gen.fromIterable(input)) { case (string, expected) =>
          val output = MustacheExpression.syntax.parseString(string)
          assert(output)(isRight(equalTo(expected)))
        }
      },
      test("encoding") {
        val input = List(
          MustacheExpression("a")           -> "{{a}}",
          MustacheExpression("a", "b")      -> "{{a.b}}",
          MustacheExpression("a", "b", "c") -> "{{a.b.c}}",
        )
        checkAll(Gen.fromIterable(input)) { case (input, expected) =>
          val output = MustacheExpression.syntax.printString(input)
          assert(output)(isRight(equalTo(expected)))
        }
      },
      test("evaluate") {
        val input = List(
          "{{a}}"     -> DynamicValue(Map("a" -> 1)),
          "{{a.b}}"   -> DynamicValue(Map("a" -> Map("b" -> 1))),
          "{{a.b.c}}" -> DynamicValue(Map("a" -> Map("b" -> Map("c" -> 1)))),
        )

        checkAll(Gen.fromIterable(input)) { case (mustache, input) =>
          val output = MustacheExpression.evaluate(mustache, input).toZIO
          assertZIO(output)(equalTo("1"))
        }

      },
      test("syntax") {
        val input = List(
          "ab"          -> Mustache(txt("ab")),
          "ab{{c.d}}"   -> Mustache(txt("ab"), prm("c", "d")),
          "ab{{c.d}}ef" -> Mustache(txt("ab"), prm("c", "d"), txt("ef")),
        )

        checkAll(Gen.fromIterable(input)) { case (string, template) =>
          val output  = Mustache.syntax.parseString(string)
          val encoded = Mustache.syntax.printString(template)
          assert(output)(isRight(equalTo(template))) && assert(encoded)(isRight(equalTo(string)))
        }
      },
      test("evaluate") {
        val input = List(
          "x{{a}}"             -> DynamicValue(Map("a" -> 1))                         -> "x1",
          "{{a.b}}y"           -> DynamicValue(Map("a" -> Map("b" -> 1)))             -> "1y",
          "x{{a.b.c}}y"        -> DynamicValue(Map("a" -> Map("b" -> Map("c" -> 1)))) -> "x1y",
          "x{{a}}y{{b}}z{{c}}" -> DynamicValue(Map("a" -> 1, "b" -> 2))               -> s"x1y2z{{c}}",
        )

        checkAll(Gen.fromIterable(input)) { case template -> input -> expected =>
          for {
            mustache <- Mustache.evaluate(template, input).toZIO
            actual   <- mustache.encode.toZIO
          } yield assertTrue(actual == expected)
        }
      },
    )
}
