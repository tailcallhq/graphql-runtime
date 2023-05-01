package tailcall.runtime

import tailcall.runtime.service.HttpCache
import tailcall.runtime.service.HttpCache.dateFormat
import zio._
import zio.http.Response
import zio.http.model.Headers
import zio.http.model.Headers.Header
import zio.test.Assertion._
import zio.test._

import java.time.Instant

object HttpCacheSpec extends ZIOSpecDefault {
  def spec =
    suite("HttpCacheSpec Cache-Control")(
      test("ttl") {
        val ttl      = HttpCache.ttl(Response.ok.addHeaders(headers = Header("Cache-Control", "max-age=1000")))
        val expected = Some(Duration.fromSeconds(1000))
        assert(ttl)(equalTo(expected))
      },
      test("ttl cache-control") {
        val ttl      = HttpCache.ttl(Response.ok.addHeaders(headers = Header("cache-control", "max-age=1000")))
        val expected = Some(Duration.fromSeconds(1000))
        assert(ttl)(equalTo(expected))
      },
      test("ttl cache-control private") {
        val ttl      = HttpCache.ttl(Response.ok.addHeaders(headers = Header("cache-control", "max-age=1000, private")))
        val expected = None
        assert(ttl)(equalTo(expected))
      },
      test("expires -1") {
        val ttl      = HttpCache.ttl(Response.ok.addHeaders(headers = Header("expires", "-1")))
        val expected = None
        assert(ttl)(equalTo(expected))
      },
      test("cache-control and expires") {
        lazy val expiry = Instant.now().plusSeconds(1000).toString
        val ttl         = HttpCache.ttl(
          Response.ok.addHeaders(headers = Headers(Header("expires", expiry), Header("cache-control", "max-age=2000")))
        )
        val expected    = Some(Duration.fromSeconds(2000))
        assert(ttl)(equalTo(expected))
      },
      test("expires after 1000 second") {
        val now        = Instant.parse("2021-01-01T00:00:00Z")
        val headerTime = dateFormat.format(now.toEpochMilli + 1000000L)
        val p          = HttpCache.ttl(Response.ok.addHeaders(headers = Header("expires", headerTime)), now)
        val expected   = Some(Duration.fromSeconds(1000))
        assert(p)(equalTo(expected))
      },
    )

}
