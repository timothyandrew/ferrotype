import com.google.gson.annotations.SerializedName
import java.time.LocalDate
import java.time.ZoneOffset
import java.util.*

data class VideoMetadata(val cameraMake: String)
data class PhotoMetadata(val cameraMake: String)

data class MediaMetadata(
    val creationTime: Date,
    val photo: PhotoMetadata?,
    val video: VideoMetadata?,
)

data class MediaItem(
    val id: String,
    val baseUrl: String,
    val mimeType: String,
    @SerializedName("mediaMetadata")
    val metadata: MediaMetadata,
    val filename: String
) {
    fun creationTime(): LocalDate = metadata.creationTime.toInstant().atOffset(ZoneOffset.UTC).toLocalDate()
}