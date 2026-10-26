The release build places studio-home.apk here (built from launcher/ by CI or by
launcher\gradlew.bat assembleRelease, then copied as studio-home.apk).
The desktop app reports "launcher not bundled" when it is missing; it never downloads one.
