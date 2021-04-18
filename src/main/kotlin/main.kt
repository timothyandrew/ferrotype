import dl.MediaDownloadService
import dl.MetadataDownloadService
import io.github.cdimascio.dotenv.dotenv
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import org.slf4j.LoggerFactory

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

    val getAccessToken = Channel<String>()
    val getMetadataPage = Channel<List<MediaItem>>()

    // TODO: Download images
    // TODO: Recheck motion photo logic
    // TODO: Refresh token in 55 minutes
    // TODO: Auth: handle non-200s
    // TODO: Metrics
    // TODO: Don't crash when we hit the rate limit
    // TODO: Retry on 500s

    coroutineScope {
        try {
            launch { AuthService(credentials, getAccessToken).start() }
            launch { MetadataDownloadService(getMetadataPage, getAccessToken).start() }
            launch { MediaDownloadService(getMetadataPage, "/tmp/ferrotype", "/home/tim/ferrotype-db/non-motion-photos").start() }
        } catch (e: Exception) {
            log.error("Catastrophic failure!", e)
        }
    }

    log.info("Shutting down...")
}