# Run Flutter app on connected Android device with JDK 17
$env:JAVA_HOME = "C:\Program Files\ojdkbuild\java-17-openjdk-17.0.3.0.6-1"
Set-Location $PSScriptRoot
flutter run -d android
