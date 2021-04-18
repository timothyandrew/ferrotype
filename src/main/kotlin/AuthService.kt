import com.google.gson.annotations.SerializedName
import io.ktor.client.*
import io.ktor.client.engine.cio.*
import io.ktor.client.features.json.*
import io.ktor.client.request.forms.*
import io.ktor.http.*
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import org.slf4j.LoggerFactory
import kotlin.time.ExperimentalTime
import kotlin.time.seconds

data class Credentials(val clientId: String, val clientSecret: String, val refreshToken: String? = null) {
    fun withRefreshToken(token: String): Credentials = Credentials(clientId, clientSecret, token)
}

data class GetTokenResponse(
    @SerializedName("access_token")
    val accessToken: String,
    @SerializedName("expires_in")
    val expiresIn: Int,
    @SerializedName("refresh_token")
    val refreshToken: String,
    val scope: String
)

data class GetTokenViaRefreshResponse(
    @SerializedName("access_token")
    val accessToken: String,
    @SerializedName("expires_in")
    val expiresIn: Int,
    val scope: String
)


class AuthService(private var credentials: Credentials, private val send: Channel<String>) {
    private val log = LoggerFactory.getLogger("AuthService")

    @Volatile
    private var accessToken: String? = null

    @OptIn(ExperimentalTime::class)
    suspend fun start() = coroutineScope {
        launch {
            if (credentials.refreshToken == null) authorizeInitial() else refresh()
            log.info("Fetched access token")
        }

        launch {
            delay(2.seconds.toLongMilliseconds())
            while(true) {
                val token = accessToken

                if(token == null) {
                    log.warn("Don't have an access token yet; waiting 5 seconds")
                    delay(5.seconds.toLongMilliseconds())
                } else {
                    send.send(token)
                }
            }
        }
    }

    private val client = HttpClient(CIO) {
        install(JsonFeature) {
            serializer = GsonSerializer()
        }
    }

    private val authorizeUrl = URLBuilder().apply {
        protocol = URLProtocol.HTTPS
        host = "accounts.google.com"
        path(listOf("o", "oauth2", "v2", "auth"))

        parameters.append("client_id", credentials.clientId)
        parameters.append("redirect_uri", "http://example.com")
        parameters.append("response_type", "code")
        parameters.append("scope", "https://www.googleapis.com/auth/photoslibrary.readonly")
        parameters.append("access_type", "offline")
        parameters.append("state", "random")
        parameters.append("include_granted_scopes", "true")
        parameters.append("prompt", "consent")
    }

    private suspend fun getToken(code: String): GetTokenResponse {
        val params = ParametersBuilder().apply {
            append("client_id", credentials.clientId)
            append("client_secret", credentials.clientSecret)
            append("code", code)
            append("grant_type", "authorization_code")
            append("redirect_uri", "http://example.com")
        }

        return client.submitForm<GetTokenResponse>(params.build()) {
            url {
                protocol = URLProtocol.HTTPS
                host = "oauth2.googleapis.com"
                path(listOf("token"))
            }
        }
    }

    private suspend fun getTokenViaRefresh(refreshToken: String): GetTokenViaRefreshResponse {
        val params = ParametersBuilder().apply {
            append("client_id", credentials.clientId)
            append("client_secret", credentials.clientSecret)
            append("refresh_token", refreshToken)
            append("grant_type", "refresh_token")
        }

        return client.submitForm<GetTokenViaRefreshResponse>(params.build()) {
            url {
                protocol = URLProtocol.HTTPS
                host = "oauth2.googleapis.com"
                path(listOf("token"))
            }
        }
    }

    private suspend fun authorizeInitial() {
        println("One-time auth setup")
        println("-------------------")
        println("1. Navigate to this URL and log in:")
        println("   ${authorizeUrl.buildString()}")
        print("2. Paste the code you're given here: ")

        val code = readLine() ?: throw Error("Can't authorize without a `code`")
        val response = getToken(code)

        accessToken = response.accessToken
        credentials = credentials.withRefreshToken(response.refreshToken)
        println("3. Save this refresh token for next time: ${response.refreshToken}")
    }

    private suspend fun refresh() {
        val refreshToken = credentials.refreshToken ?: throw Error("Can't refresh without a refresh token")
        val response = getTokenViaRefresh(refreshToken)
        accessToken = response.accessToken
    }
}