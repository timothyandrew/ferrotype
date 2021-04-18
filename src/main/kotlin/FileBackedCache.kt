import kotlinx.coroutines.*
import org.slf4j.LoggerFactory
import java.io.File
import java.nio.charset.Charset
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicBoolean
import kotlin.time.ExperimentalTime
import kotlin.time.minutes

// Extremely simplistic in memory cache, backed by a single file on disk
class FileBackedCache(private val filename: String) {
    private val cache = mutableSetOf<String>()
    private val log = LoggerFactory.getLogger("FileBackedCache")
    private val context = Executors.newSingleThreadExecutor().asCoroutineDispatcher()

    @Volatile
    private var needsFlush = AtomicBoolean(false)

    @OptIn(ExperimentalTime::class)
    suspend fun initialize()  = withContext(context) {
        loadFromFile()

        while (true) {
            delay(5.minutes.toLongMilliseconds())

            if (needsFlush.compareAndExchange(true, false)) {
                log.info("Flushing file-backed cache (${cache.size} entries) to disk at $filename")
                writeToFile()
            }
        }
    }

    private suspend fun loadFromFile() = withContext(context) {
        log.info("Loading file-backed cache from $filename...")
        val data = File(filename).readLines(Charset.defaultCharset())
        cache.clear()
        cache.addAll(data)
        log.info("Loaded file-backed cache (${cache.size} entries) from $filename")
    }

    private suspend fun writeToFile() = withContext(context) {
        log.info("Writing file-backed cache to disk...")
        val data = cache.joinToString("\n")
        File(filename).writeText(data)
    }

    // NOTE: This doesn't use `context` because this function is going to be called fairly often, and
    //       we can afford to trade off false negatives for performance.
    fun contains(element: String): Boolean = cache.contains(element)

    suspend fun add(element: String) = withContext(context) {
        cache.add(element)
        needsFlush.set(true)
    }
}