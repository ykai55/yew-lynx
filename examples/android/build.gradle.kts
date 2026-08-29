import java.util.concurrent.TimeUnit

buildscript {
    repositories {
        google()
        mavenCentral()
    }
    dependencies {
        classpath("com.android.tools.build:gradle:7.4.2")
        classpath("org.jetbrains.kotlin:kotlin-gradle-plugin:1.8.22")
    }
}

val lynxVersion = "0.0.1-0df14207"
val repositoryRoot = rootDir.parentFile.parentFile

allprojects {
    repositories {
        maven {
            url = uri(
                repositoryRoot.resolve(
                    "third_party/lynx/platform/android/build/release/$lynxVersion"
                )
            )
        }
        google()
        mavenCentral()
    }

    dependencyLocking {
        lockAllConfigurations()
    }
    configurations.all {
        resolutionStrategy {
            cacheChangingModulesFor(0, TimeUnit.SECONDS)
        }
    }
}

extra["lynxVersion"] = lynxVersion

tasks.register<Delete>("clean") {
    delete(rootProject.buildDir)
    delete(subprojects.map { it.buildDir })
}
