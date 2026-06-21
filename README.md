# 42Host

GUI для запуска и управления локальными серверами Minecraft.
**Rust + GTK4 + libadwaita**. Поддерживаются Linux (Wayland/X11) и Windows 10/11.

## Возможности

- Список серверов с живыми RAM / CPU / uptime, адаптивный интерфейс (как Nautilus).
- Запуск / остановка / перезапуск, автоперезапуск при краше, приём EULA.
- Консоль с вводом команд, историей и автоскроллом.
- Плагины: сетка карточек, вкл/выкл, удаление, drag-and-drop `.jar`, открытие конфига в терминале.
- Игроки онлайн с действиями (kick / ban / op).
- Редактор `server.properties` (часть параметров применяется без перезапуска).
- Настройки темы, шрифта, терминала и редактора.

## Сборка на Linux

```sh
# зависимости (Arch): sudo pacman -S gtk4 libadwaita
cargo run --release
```

Требуется Rust ≥ 1.75, GTK ≥ 4.12, libadwaita ≥ 1.5.

## Сборка на Windows

Самый прямой вариант — MSYS2 UCRT64:

1. Установите [MSYS2](https://www.msys2.org/) и откройте терминал **MSYS2 UCRT64**.
2. Установите Rust, GTK4, libadwaita и инструменты сборки:

```sh
pacman -Syu
pacman -S --needed mingw-w64-ucrt-x86_64-gcc \
  mingw-w64-ucrt-x86_64-gtk4 \
  mingw-w64-ucrt-x86_64-libadwaita \
  mingw-w64-ucrt-x86_64-pkgconf \
  mingw-w64-ucrt-x86_64-rust
```

3. В том же терминале соберите и запустите приложение:

```sh
cargo run --release
```

Готовый файл будет в `target/release/42host.exe`. Для запуска GTK DLL должны быть доступны через `PATH`; при установке выше это выполняется в терминале UCRT64. Java также должна быть установлена и доступна как `java.exe`.

Windows использует `cmd.exe`, `notepad.exe` и `explorer.exe` как стандартные внешние приложения. Их можно заменить в настройках 42Host, например на `wt.exe` и `code`.

Обе платформы проверяются в GitHub Actions.

## Данные

Linux: `~/.config/42host/` — серверы и настройки, `~/.cache/42host/logs/` — логи.

Windows: `%APPDATA%\42host\` — серверы и настройки, `%LOCALAPPDATA%\42host\logs\` — логи.

## Лицензия

[MIT](LICENSE) · © 2026 wioletowa
