import kotlinx.coroutines.*
import kotlinx.coroutines.channels.ReceiveChannel
import org.slf4j.LoggerFactory
import java.io.File
import java.util.*
import java.util.concurrent.Executors
import kotlin.time.ExperimentalTime
import kotlin.time.hours
import kotlin.time.minutes

// Maintain a cache of media items on a faster disk for easier
// external access
class SSDCacheService(
    private val sourceDir: String,
    private val destinationDir: String,
    private val accept: ReceiveChannel<File>
) {
    // How many media items to copy over?
    private val count = 10000
    @OptIn(ExperimentalTime::class)
    private val interval = 24.hours

    private val log = LoggerFactory.getLogger("SSDCacheService")
    private val cache = mutableSetOf<File>()
    private val context = Executors.newSingleThreadExecutor().asCoroutineDispatcher()


    private fun getFilename(file: File): String {
        val dirName = file.parentFile.name
        val current = System.currentTimeMillis()
        val extension = file.extension
        return "media-${current}-$dirName-${UUID.randomUUID()}.$extension"
    }

    @OptIn(ExperimentalTime::class)
    suspend fun start() = coroutineScope {
        log.info("Starting cache service...")

        // NOTE: Using the same single-threaded context for all coroutines guarantees
        //       that access to `cache` is synchronized

        // NOTE: Use `withContext` here so the subsequent coroutines aren't started until this one is done
        withContext(context) {
            log.info("Walking $sourceDir to build an in-memory cache of file locations")
            cache.addAll(File(sourceDir).walk().filter { it.isFile }.toSet())
            log.info("Cache built from $sourceDir with ${cache.size} entries")
        }

        launch(context) { while(true) cache.add(accept.receive()) }

        launch(context) {
            while(true) {
                log.info("Going to copy $count files to $destinationDir (${cache.size} files in the cache)")
                val files = cache.shuffled().take(count)

                withContext(Dispatchers.IO) {
                    // WARNING: DANGEROUS!
                    val destination = File(destinationDir).also {
                        if (it.exists()) it.deleteRecursively()
                        it.mkdirs()
                    }

                    files.forEach { it.copyTo(destination.resolve(File(getFilename(it)))) }
                }

                log.info("Copied $count files to $destinationDir. Going to wait ${interval.inHours} hours before doing this again.")
                delay(interval.toLongMilliseconds())
            }
        }
    }
}