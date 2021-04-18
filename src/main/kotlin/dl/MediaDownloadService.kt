package dl

import MediaItem
import io.ktor.client.*
import io.ktor.client.call.*
import io.ktor.client.engine.cio.*
import io.ktor.client.request.*
import io.ktor.client.statement.*
import io.ktor.http.*
import kotlinx.coroutines.*
import kotlinx.coroutines.channels.Channel
import org.slf4j.LoggerFactory
import java.io.File
import java.nio.charset.Charset
import java.time.format.DateTimeFormatter
import kotlin.time.ExperimentalTime
import kotlin.time.minutes

// TRICKY: Google's API documentation is incorrect for motion photos.
//   - "Motion photos contain both photo and video elements" ← THIS IS NOT TRUE
//   - A motion photo is indistinguishable from a regular photo based on the metadata response (it doesn't have the `video` parameter set).
//   - The only way to detect a motion photo is to append `=dv` to the baseUrl and attempt to download it. If you get something back, it's a motion photo.
//   - The "something" you get back is always a video file, even though the documentation (incorrectly) says that you get back a ZIP file for iOS motion photos.
//
//   To get around this:
//     - We use a cache to keep track of photos we know are _not_ motion photos, so they don't need to be checked a second time.
//     - We can't use the filesystem for this check because there's no way to distinguish between a non-motion photo and a motion
//       photo whose video component hasn't downloaded yet.
class MediaDownloadService(
    private val getMediaItems: Channel<List<MediaItem>>,
    private val prefix: String,
    private val nonMotionPhotoCachePath: String
) {
    private val log = LoggerFactory.getLogger("MediaDownloadService")
    private val client = HttpClient(CIO) { expectSuccess = false }

    // A set of `MediaItem` ids representing media items we know are _not_ motion photos
    // TODO: Is @Volatile guaranteed here?
    private val nonMotionPhotoCache = mutableSetOf<String>()

    private fun loadNonMotionPhotoCache() {
        log.info("Loading non-motion photo cache")
        val data = File(nonMotionPhotoCachePath).readLines(Charset.defaultCharset())
        nonMotionPhotoCache.clear()
        nonMotionPhotoCache.addAll(data)
        log.info("Loaded non-motion photo cache")
    }

    private fun writeNonMotionPhotoCache() {
        log.info("Writing non-motion photo cache to disk")
        val data = nonMotionPhotoCache.joinToString("\n")
        File(nonMotionPhotoCachePath).writeText(data)
    }

    private fun itemDir(item: MediaItem, prefix: File): File {
        val year = item.creationTime().year.toString()
        val date = item.creationTime().format(DateTimeFormatter.ofPattern("yyyy-MM-dd"))
        val dir =  prefix.resolve(File(year)).resolve(File(date))
        if (!dir.exists()) dir.mkdirs()
        return dir
    }

    private suspend fun downloadUrl(url: String, dir: File, item: MediaItem) {
        val filename = if (url.contains("=dv")) "${item.id}.mp4" else "${item.id}.jpg"
        val file = dir.resolve(File(filename))

        if (file.exists()) return
        val response: HttpResponse = client.get(url)

        when {
            response.status == HttpStatusCode.OK -> {
                val data = response.receive<ByteArray>()
                file.writeBytes(data)
            }
            response.status == HttpStatusCode.Unauthorized -> throw Error("Unauthorized!")
            response.status == HttpStatusCode.TooManyRequests -> throw Error("We've hit the rate limit, try again later! (rate limits reset at midnight PT)")
            // We checked to see if a photo was also a motion photo, a 404 means the answer was NO
            response.status == HttpStatusCode.NotFound -> nonMotionPhotoCache.add(item.id)
            response.status.value in (500..599) -> TODO("Retry logic")
            else -> throw Error("Unknown response code (${response.status.value}) when downloading a media item")
        }
    }

    // Download the file(s) underlying a given MediaItem.
    //   - Videos and regular photos are backed by a single file each
    //   - Motion photos are backed by two files, a photo and a video
    private suspend fun downloadItem(item: MediaItem) = coroutineScope {
        val dir = itemDir(item, File(prefix))

        val urls = when {
            // Definitely a regular photo
            nonMotionPhotoCache.contains(item.id) -> listOf("${item.baseUrl}=d")
            // Photo or a motion photo
            item.metadata.photo != null -> listOf("${item.baseUrl}=d", "${item.baseUrl}=dv")
            // Definitely a video
            item.metadata.video != null -> listOf("${item.baseUrl}=dv")
            else -> throw Error("Invalid API response; file isn't a photo OR a video")
        }

        urls.map { async { downloadUrl(it, dir, item) } }.awaitAll()
    }

    @OptIn(ExperimentalTime::class)
    suspend fun start() = coroutineScope {
        val mainLoop = launch {
            while (true) {
                val items = getMediaItems.receive()
                items.map { async { downloadItem(it) } }.awaitAll()
            }
        }

        val loadCache = launch(Dispatchers.IO) { loadNonMotionPhotoCache() }

        val saveCachePeriodically = launch(Dispatchers.IO) {
            while (true) {
                delay(5.minutes.toLongMilliseconds())
                writeNonMotionPhotoCache()
            }
        }
    }
}