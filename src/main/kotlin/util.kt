import kotlinx.coroutines.delay
import org.slf4j.LoggerFactory
import java.time.LocalDate
import java.time.LocalDateTime
import java.time.ZoneId
import java.time.temporal.ChronoUnit
import kotlin.time.ExperimentalTime
import kotlin.time.milliseconds

@OptIn(ExperimentalTime::class)
suspend fun delayUntilMidnightPT() {
    val log = LoggerFactory.getLogger("util/delayUntilMidnightPT")

    val target = LocalDate.now().plusDays(1).atStartOfDay(ZoneId.of("America/Los_Angeles")).plusMinutes(30)
    val delayMs = LocalDateTime.now().atZone(ZoneId.systemDefault()).until(target, ChronoUnit.MILLIS)
    log.info("Going to delay until midnight PT (${delayMs.milliseconds.inHours} hours away) for the rate limit to reset")

    delay(delayMs)
}