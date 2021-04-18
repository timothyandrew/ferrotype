package dl

import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.channels.ReceiveChannel
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.launch
import org.slf4j.LoggerFactory
import kotlin.time.ExperimentalTime

// KLUDGE: Doing this because Kotlin doesn't have union types (and I can't think of an
//         alternative that's this succinct.
enum class Metric {
    // Metrics
    MEDIA_ITEM_DL, MEDIA_ITEM_SKIPPED_EXISTS,
    MOTION_PHOTO_CACHE_HIT, RETRY_5XX,
    RATE_LIMIT, METADATA_PAGE_DL,

    // Commands
    FLUSH
}

class MetricsService(private val accept: ReceiveChannel<Metric>) {
    private val db = mutableMapOf<Metric, Int>()
    private val log = LoggerFactory.getLogger("MetricsService")

    @OptIn(ExperimentalTime::class)
    suspend fun start() = coroutineScope {
        log.info("Starting metrics service...")

        launch {
            while (true) {
                val metric = accept.receive()

                if (metric == Metric.FLUSH) flush()
                else db[metric] = (db[metric] ?: 0) + 1
            }
        }

//        launch {
//            while (true) {
//                delay(30.minutes.toLongMilliseconds())
//                flush()
//            }
//        }
    }

    fun flush() {
        log.info(db.toString())
    }
}