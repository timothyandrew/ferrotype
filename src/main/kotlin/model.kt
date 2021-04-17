data class VideoMetadata(val cameraMake: String)
data class PhotoMetadata(val cameraMake: String)

data class MediaMetadata(
    val creationTime: String,
    val photo: PhotoMetadata?,
    val video: VideoMetadata?,
)

data class MediaItem(
    val id: String,
    val baseUrl: String,
    val mimeType: String,
    val mediaMetadata: MediaMetadata,
    val filename: String,
)