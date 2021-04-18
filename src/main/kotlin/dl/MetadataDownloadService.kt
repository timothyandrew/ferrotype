package dl

import MediaItem
import io.ktor.client.*
import io.ktor.client.call.*
import io.ktor.client.engine.cio.*
import io.ktor.client.features.json.*
import io.ktor.client.request.*
import io.ktor.client.statement.*
import io.ktor.http.*
import kotlinx.coroutines.channels.Channel
import org.slf4j.LoggerFactory

data class GetMetadataPageResponse(
    val mediaItems: List<MediaItem>,
    val nextPageToken: String?
)

class MetadataDownloadService(
    private val send: Channel<List<MediaItem>>,
    private val getAccessToken: Channel<String>
) {
    private val log = LoggerFactory.getLogger("MetadataDownloadService")

    private val client = HttpClient(CIO) {
        expectSuccess = false

        install(JsonFeature) {
            serializer = GsonSerializer() {
                setDateFormat("yyyy-MM-dd'T'HH:mm:ssz")
            }
        }
    }

    private suspend fun getMetadataPage(page: String?): HttpResponse {
        val accessToken = getAccessToken.receive()
        val url = URLBuilder().apply {
            protocol = URLProtocol.HTTPS
            host = "photoslibrary.googleapis.com"
            path(listOf("v1", "mediaItems"))

            parameters.append("pageSize", "100")
            if (page != null) parameters.append("pageToken", page)
        }


        log.info("Going to download a metadata page")

        return client.get(url.build()) {
            headers {
                append("Authorization", "Bearer $accessToken")
            }
        }
    }

    suspend fun start() {
        var pageToken: String? = null

        while (true) {
            val response = getMetadataPage(pageToken)

            when {
                response.status == HttpStatusCode.OK -> {
                    val page = response.receive<GetMetadataPageResponse>()
                    send.send(page.mediaItems)

                    if (page.nextPageToken == null) {
                        log.info("All metadata pages downloaded")
                        break
                    } else {
                        pageToken = page.nextPageToken
                    }
                }
                response.status == HttpStatusCode.Unauthorized -> throw Error("Unauthorized!")
                response.status == HttpStatusCode.TooManyRequests -> throw Error("We've hit the rate limit, try again later! (rate limits reset at midnight PT)")
                response.status.value in (500..599) -> TODO("Retry logic")
                else -> throw Error("Unknown response code (${response.status.value}) when downloading a page of metadata")
            }
        }
    }
}