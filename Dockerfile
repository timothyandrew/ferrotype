FROM gradle:6.8.3-jdk11 AS gradle

WORKDIR /usr/local/app
RUN mkdir -p /home/gradle/deps/.gradle

ADD gradle /usr/local/app/gradle
ADD gradle.properties /usr/local/app/
ADD gradlew /usr/local/app/
ADD settings.gradle.kts /usr/local/app/
ADD build.gradle.kts /usr/local/app/

# Download dependencies
RUN gradle -i --gradle-user-home /home/gradle/deps/.gradle clean build || return 0

ADD src /usr/local/app/src
RUN gradle -i --gradle-user-home /home/gradle/deps/.gradle shadowJar

FROM openjdk:15-slim-buster
COPY --from=gradle /usr/local/app/build/libs/ferrotype-1.0-SNAPSHOT-all.jar /usr/local/app/app.jar
WORKDIR /usr/local/app
CMD ["java", "-Xmx4096m", "-Xms1024m", "-XX:+UseZGC", "-jar", "app.jar"]