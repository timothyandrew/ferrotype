package dl

import FileBackedCache
import MediaItem
import delayUntilMidnightPT
import io.ktor.client.*
import io.ktor.client.call.*
import io.ktor.client.engine.cio.*
import io.ktor.client.request.*
import io.ktor.client.statement.*
import io.ktor.http.*
import kotlinx.coroutines.*
import kotlinx.coroutines.channels.ReceiveChannel
import kotlinx.coroutines.channels.SendChannel
import org.slf4j.LoggerFactory
import java.io.File
import java.time.format.DateTimeFormatter

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
    private val getMediaItems: ReceiveChannel<List<MediaItem>>,
    private val prefix: String,
    private val nonMotionPhotoCachePath: String,
    private val sendMetric: SendChannel<Metric>,
    private val sendDownloadedFile: SendChannel<File>
) {
    private val log = LoggerFactory.getLogger("MediaDownloadService")
    private val client = HttpClient(CIO) { expectSuccess = false }
    private val maxRetries = 3

    // A set of `MediaItem` ids representing media items we know are _not_ motion photos
    private val nonMotionPhotoCache = FileBackedCache(nonMotionPhotoCachePath)

    private fun itemDir(item: MediaItem, prefix: File): File {
        val year = item.creationTime().year.toString()
        val date = item.creationTime().format(DateTimeFormatter.ofPattern("yyyy-MM-dd"))
        val dir =  prefix.resolve(File(year)).resolve(File(date))
        if (!dir.exists()) dir.mkdirs()
        return dir
    }

    private suspend fun downloadUrl(url: String, dir: File, item: MediaItem, retryCount: Int = 0) {
        if (retryCount >= maxRetries) {
            log.warn("Exceeded maximum retries while downloading item with id ${item.id}")
            return
        }

        val filename = if (url.contains("=dv")) "${item.id}.mp4" else "${item.id}.jpg"
        val file = dir.resolve(File(filename))

        if (file.exists()) {
            sendMetric.send(Metric.MEDIA_ITEM_SKIPPED_EXISTS)
            return
        }

        val response: HttpResponse = client.get(url)

        when {
            response.status == HttpStatusCode.OK -> {
                val data = response.receive<ByteArray>()
                file.writeBytes(data)
                sendMetric.send(Metric.MEDIA_ITEM_DL)
                sendDownloadedFile.send(file)
            }
            response.status == HttpStatusCode.Unauthorized -> throw Error("Unauthorized!")
            response.status == HttpStatusCode.TooManyRequests -> {
                sendMetric.send(Metric.RATE_LIMIT)
                delayUntilMidnightPT()
                downloadUrl(url, dir, item, retryCount + 1)
            }
            // We checked to see if a photo was also a motion photo, a 404 means the answer was NO
            response.status == HttpStatusCode.NotFound -> nonMotionPhotoCache.add(item.id)
            response.status.value in (500..599) -> {
                sendMetric.send(Metric.RETRY_5XX)
                downloadUrl(url, dir, item, retryCount + 1)
            }
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
            nonMotionPhotoCache.contains(item.id) -> {
                sendMetric.send(Metric.MOTION_PHOTO_CACHE_HIT)
                listOf("${item.baseUrl}=d")
            }
            // Photo or a motion photo
            item.metadata.photo != null -> listOf("${item.baseUrl}=d", "${item.baseUrl}=dv")
            // Definitely a video
            item.metadata.video != null -> listOf("${item.baseUrl}=dv")
            else -> throw Error("Invalid API response; file isn't a photo OR a video")
        }

        for (url in urls) launch(Dispatchers.IO) { downloadUrl(url, dir, item) }
    }

    suspend fun start() = coroutineScope {
        log.info("Starting media download service...")

        // TODO: Is there a better pattern here?
        launch { nonMotionPhotoCache.initialize() }

        // TODO: Don't start receiving until the non-motion photo cache is ready
        launch {
            while (true) {
                val items = getMediaItems.receive()
                items.map { async { downloadItem(it) } }.awaitAll()
            }
        }
    }
}