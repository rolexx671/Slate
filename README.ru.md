# Slate RU

Русская версия [Slate](https://github.com/Soflutionltd/Slate) для macOS Apple Silicon.
Основа: `adfb0722263365eb8d11b2ffdb1c3e91a9fd7889`. Версия сборки: `0.3.138-ru.1`.

Русский включён при первом запуске независимо от языка macOS. Переведены интерфейс,
подсказки, меню, окна, доступные пользователю сообщения и инструкции ИИ-помощника.
Содержимое PDF, имена файлов и названия шрифтов сохраняются без перевода.

Приложение называется **Slate RU**, идентификатор — `com.rolexx671.slate.ru`.
Оно хранит настройки отдельно от оригинала, не меняет обработчик PDF по умолчанию
и не устанавливает обновления оригинального Slate. Обновления выполняются вручную
путём сборки новой версии этой ветки. Подпись локальная (ad hoc), без нотариального
заверения Apple. Системные окна используют русскую локализацию macOS; названия,
предоставляемые драйвером принтера или внешними сервисами, определяются ими.

## Повторная сборка

Нужны Xcode Command Line Tools, Rust stable и Tauri CLI 2:

```sh
xcode-select --install
# Установите Rust через https://rustup.rs, затем:
cargo install tauri-cli --version '^2' --locked
git clone --branch russian-localization https://github.com/rolexx671/Slate.git
cd Slate
bash scripts/build-ru.sh
```

Сценарий загружает PDFium `chromium/7891`, проверяет SHA-256, собирает Rust-модули
и приложение. Готовый пакет: `src-tauri/target/release/bundle/macos/Slate RU.app`.
Скопируйте его в `/Applications`. Сценарий `scripts/release.sh` относится к оригиналу;
для русской версии используйте только `scripts/build-ru.sh`.

## Проверки

```sh
node --check src-tauri/frontend-dist/app.js
node tests/localization.mjs
ALTO_PDFIUM_DIR="$PWD/src-tauri" cargo test --release --manifest-path src-tauri/Cargo.toml --test pdf_engine_tests --test print_label_tests
```

`ru.js` содержит русский словарь и правила множественного числа. Новые пользовательские
строки добавляйте в словарь и вызывайте через `t()`. Технические идентификаторы команд
и форматов PDF не переводите. Подробности внутренних ошибок остаются в диагностике,
а пользователь получает русское сообщение.

Лицензия: AGPL-3.0-or-later. Авторские уведомления оригинала сохранены в LICENSE и NOTICE.
