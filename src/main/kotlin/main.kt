import dl.MediaDownloadService
import dl.MetadataDownloadService
import dl.Metric
import dl.MetricsService
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import org.slf4j.LoggerFactory
import java.io.File
import kotlin.time.ExperimentalTime

@ExperimentalTime
fun main() = runBlocking<Unit> {
    val log = LoggerFactory.getLogger("mainLoop")

    val downloadPath = System.getenv("FERROTYPE_DOWNLOAD_PATH") ?: throw Error("FERROTYPE_DOWNLOAD_PATH not set")
    val nonMotionPhotoCachePath = System.getenv("FERROTYPE_NON_MOTION_PHOTO_CACHE_PATH") ?: throw Error("FERROTYPE_NON_MOTION_PHOTO_CACHE_PATH not set")
    val ssdCachePath = System.getenv("FERROTYPE_SSD_CACHE_PATH")

    val clientId = System.getenv("FERROTYPE_CLIENT_ID") ?: throw Error("FERROTYPE_CLIENT_ID not set")
    val clientSecret = System.getenv("FERROTYPE_CLIENT_SECRET") ?: throw Error("FERROTYPE_CLIENT_SECRET not set")
    val refreshToken = System.getenv("FERROTYPE_REFRESH_TOKEN") ?: null
    val credentials = Credentials(clientId, clientSecret, refreshToken)

    val sendDownloadedFile = Channel<File>(10000)
    val sendMetric = Channel<Metric>(1000)
    val getAccessToken = Channel<String>()
    val getMetadataPage = Channel<List<MediaItem>>()

    // TODO: Start subsequent runs at a given time of day
    // TODO: Backoff when retrying

    coroutineScope {
        // TODO: Thin some of these these parameters out, especially the `MediaDownloadService`
        try {
            launch { AuthService(credentials, getAccessToken).start() }
            launch { MetadataDownloadService(getMetadataPage, getAccessToken, sendMetric).start() }
            launch { MediaDownloadService(getMetadataPage, downloadPath, nonMotionPhotoCachePath, sendMetric, sendDownloadedFile).start() }
            launch { MetricsService(sendMetric).start() }
            launch { SSDCacheService(downloadPath, ssdCachePath, sendDownloadedFile).start() }
        } catch (e: Exception) {
            log.error("Catastrophic failure!", e)
        }
    }

    log.info("Shutting down...")
}