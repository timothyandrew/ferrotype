package dl

import MediaItem
import delayUntilMidnightPT
import io.ktor.client.*
import io.ktor.client.call.*
import io.ktor.client.engine.cio.*
import io.ktor.client.features.json.*
import io.ktor.client.request.*
import io.ktor.client.statement.*
import io.ktor.http.*
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.delay
import org.slf4j.LoggerFactory

data class GetMetadataPageResponse(
    val mediaItems: List<MediaItem>,
    val nextPageToken: String?
)

class MetadataDownloadService(
    private val send: Channel<List<MediaItem>>,
    private val getAccessToken: Channel<String>
) {
    private val maxRetries = 10
    private val log = LoggerFactory.getLogger("MetadataDownloadService")

    private val client = HttpClient(CIO) {
        expectSuccess = false

        install(JsonFeature) {
            serializer = GsonSerializer() {
                setDateFormat("yyyy-MM-dd'T'HH:mm:ssz")
            }
        }
    }

    private suspend fun getMetadataPage(page: String?, retryCount: Int = 0): GetMetadataPageResponse {
        // TODO: Don't crash here
        if (retryCount >= maxRetries) throw Error("Exceeded maximum retries while downloading metadata page")

        val accessToken = getAccessToken.receive()
        val url = URLBuilder().apply {
            protocol = URLProtocol.HTTPS
            host = "photoslibrary.googleapis.com"
            path(listOf("v1", "mediaItems"))

            parameters.append("pageSize", "100")
            if (page != null) parameters.append("pageToken", page)
        }

        log.debug("Going to download a metadata page")

        val response = client.get<HttpResponse>(url.build()) {
            headers {
                append("Authorization", "Bearer $accessToken")
            }
        }

        return when {
            response.status == HttpStatusCode.OK -> response.receive<GetMetadataPageResponse>()
            response.status == HttpStatusCode.Unauthorized -> throw Error("Unauthorized!")
            response.status == HttpStatusCode.TooManyRequests -> {
                delayUntilMidnightPT()
                getMetadataPage(page, retryCount + 1)
            }
            response.status.value in (500..599) -> getMetadataPage(page, retryCount + 1)
            else -> throw Error("Unknown response code (${response.status.value}) when downloading a page of metadata")
        }
    }

    suspend fun start() {
        var pageToken: String? = null
        var count = 0

        while (true) {
            val page = getMetadataPage(pageToken)

            count += 1
            log.info("Metadata pages downloaded so far: $count")

            send.send(page.mediaItems)

            if (page.nextPageToken == null) {
                log.info("All metadata pages downloaded")
                break
            } else {
                pageToken = page.nextPageToken
            }
        }
    }
}