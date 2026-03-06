# Flutter App (Butterfly)

在 `d:\bt-fy\flutter_app` 下创建的 Flutter 项目，支持 Android。

## 运行到安卓手机

1. **用 JDK 17**  
   Android 构建需要 JDK 17（不要用 JDK 25）。已通过 winget 安装的路径：
   ```text
   C:\Program Files\ojdkbuild\java-17-openjdk-17.0.3.0.6-1
   ```
   建议在「系统环境变量」里把 `JAVA_HOME` 设为上述路径，或每次在终端里设置后再运行 Flutter。

2. **连接手机**  
   USB 连接安卓机并开启「USB 调试」，执行：
   ```bash
   flutter devices
   ```
   确认设备列表里有你的手机。

3. **运行**  
   在项目根目录（即本目录）执行：
   ```bash
   flutter run
   ```
   若只连了一台设备，会自动选它；多台设备时用：
   ```bash
   flutter run -d <设备ID>
   ```

## 若构建失败：NDK / 磁盘空间

若报错与 **NDK** 或 **“磁盘空间不足”** 有关：

- 在 C 盘或 Android SDK 所在盘**腾出至少约 2GB 空间**（NDK 较大）。
- 或在 **Android Studio** 中打开：**Settings → Appearance & Behavior → System Settings → Android SDK → SDK Tools**，勾选 **NDK (Side by side)**，选一个版本（如 28.x）安装。
- 然后回到本目录重新执行 `flutter run`。

## 一键运行脚本（PowerShell）

已提供 `run_android.ps1`，会设置 `JAVA_HOME` 为 JDK 17 并执行 `flutter run`。在 PowerShell 里：

```powershell
.\run_android.ps1
```
