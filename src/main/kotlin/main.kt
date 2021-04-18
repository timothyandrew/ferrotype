import dl.MediaDownloadService
import dl.MetadataDownloadService
import dl.Metric
import dl.MetricsService
import io.github.cdimascio.dotenv.dotenv
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import org.slf4j.LoggerFactory
import kotlin.time.ExperimentalTime

const val downloadPath: String = "/tmp/ferrotype"
const val cachePath: String = "/home/tim/ferrotype-db/non-motion-photos"

@ExperimentalTime
fun main() = runBlocking<Unit> {
    val log = LoggerFactory.getLogger("mainLoop")

    val env = dotenv() {
        directory = System.getProperty("user.home")
        filename = ".ferrotype"
    }

    val clientId = env["FERROTYPE_CLIENT_ID"] ?: throw Error("FERROTYPE_CLIENT_ID not set")
    val clientSecret = env["FERROTYPE_CLIENT_SECRET"] ?: throw Error("FERROTYPE_CLIENT_SECRET not set")
    val refreshToken = env["FERROTYPE_REFRESH_TOKEN"] ?: null
    val credentials = Credentials(clientId, clientSecret, refreshToken)

    val sendMetric = Channel<Metric>(1000)
    val getAccessToken = Channel<String>()
    val getMetadataPage = Channel<List<MediaItem>>()

    // TODO: Start subsequent runs at a given time of day

    coroutineScope {
        try {
            launch { AuthService(credentials, getAccessToken).start() }
            launch { MetadataDownloadService(getMetadataPage, getAccessToken, sendMetric).start() }
            launch { MediaDownloadService(getMetadataPage, downloadPath, cachePath, sendMetric).start() }
            launch { MetricsService(sendMetric).start() }
        } catch (e: Exception) {
            log.error("Catastrophic failure!", e)
        }
    }

    log.info("Shutting down...")
}