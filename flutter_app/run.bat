@echo off
rem 始终使用项目内 Gradle 缓存，避免 F:\Document\.gradle（损坏的 F 盘）
set "GRADLE_USER_HOME=%~dp0android\.gradle-home"
call flutter run %*
