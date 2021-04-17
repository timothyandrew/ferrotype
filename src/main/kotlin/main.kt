import dl.MediaDownloadService
import dl.MetadataDownloadService
import io.github.cdimascio.dotenv.dotenv
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking

fun main() = runBlocking<Unit> {
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

    coroutineScope {
        try {
            launch { AuthService(credentials, getAccessToken).start() }
            launch { MetadataDownloadService(getMetadataPage, getAccessToken).start() }
            launch { MediaDownloadService(getMetadataPage).start() }
        } catch (e: Exception) {
            println("Catastrophic failure: $e")
        }
    }

    println("Shutting down...")
}