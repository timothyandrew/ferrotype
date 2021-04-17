package dl

import kotlinx.coroutines.channels.Channel

class MediaDownloadService(private val getMediaItems: Channel<List<MediaItem>>) {
    suspend fun start() {
        while (true) {
            val metadata = getMediaItems.receive()
            println(metadata.first().id)
        }
    }
}