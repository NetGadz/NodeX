<div align="center">

  <img src="nodex-gui/assets/photo.jpg" width="140" alt="NodeX Logo" style="border-radius: 24px; box-shadow: 0 8px 24px rgba(0,0,0,0.3);" />

  # ⚡ NodeX P2P Messenger

  **P2P Messenger with a Native Rust Desktop Client and Android App**

  <p align="center">
    <a href="https://github.com/NetGadz/NodeX/releases/latest/download/NodeX-Windows-x64.exe">
      <img src="https://img.shields.io/badge/📥%20Download%20Release-NodeX--v1.0.0--x64.exe-06b6d4?style=for-the-badge&logo=windows&logoColor=white" alt="Download Windows Release" />
    </a>
  </p>

  <p align="center">
    <a href="https://github.com/rust-lang/rust"><img src="https://img.shields.io/badge/Rust-2021-orange.svg?style=for-the-badge&logo=rust&logoColor=white" alt="Rust 2021" /></a>
    <a href="https://github.com/emilk/egui"><img src="https://img.shields.io/badge/GUI-egui%20%2F%20eframe-22c55e.svg?style=for-the-badge&logo=rust&logoColor=white" alt="egui" /></a>
    <a href="https://en.wikipedia.org/wiki/Kademlia"><img src="https://img.shields.io/badge/P2P-Kademlia%20DHT-0ea5e9.svg?style=for-the-badge&logo=p2p" alt="Kademlia DHT" /></a>
    <a href="https://github.com/RustCrypto"><img src="https://img.shields.io/badge/Crypto-ChaCha20--Poly1305-8b5cf6.svg?style=for-the-badge&logo=lock" alt="Crypto" /></a>
    <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-16a34a.svg?style=for-the-badge" alt="License" /></a>
  </p>

  <p align="center">
    <b>No Phone Number Required</b> • <b>End-to-End Encryption</b> • <b>P2P Transport</b> • <b>Bootstrap-Assisted Discovery</b>
  </p>

  <p align="center">
    <a href="#-скачать-готовый-релиз-exe">📥 Скачать EXE</a> •
    <a href="#-english-overview">English</a> •
    <a href="#-описание-проекта">Русский</a> •
    <a href="#-сравнение-с-аналогами">Сравнение</a> •
    <a href="#-архитектура-системы">Архитектура</a> •
    <a href="#-быстрый-старт">Быстрый старт</a>
  </p>

</div>

---

## 📥 Скачать готовый релиз (.exe)

Для запуска NodeX на Windows **не нужно устанавливать Rust или компилировать код**! Вы можете скачать готовый исполняемый файл:

<p align="center">
  <a href="https://github.com/NetGadz/NodeX/releases/latest/download/NodeX-Windows-x64.exe" style="text-decoration: none;">
    <img src="https://img.shields.io/badge/💻%20Скачать%20NodeX%20для%20Windows%20(.exe)-v1.0.0%20(64--bit)-06b6d4?style=for-the-badge&logo=windows&logoColor=white" alt="Download Windows EXE" />
  </a>
</p>

> 💡 **Как запустить за 1 секунду:**
> 1. Скачайте файл [`NodeX-Windows-x64.exe`](https://github.com/NetGadz/NodeX/releases/latest/download/NodeX-Windows-x64.exe).
> 2. Запустите `.exe` двойным кликом — мессенджер сразу готов к работе!
> 3. Скопируйте вашу **Invite-ссылку** (`nodex://invite/...`) и отправьте другу.

---

## 📸 Интерфейс и Демонстрация (Previews)

| 💬 Чат и Голосовые Waveform | 🖼 Стеганография (Stego-Carriers) |
| :---: | :---: |
| <img src="docs/assets/demo_chat.gif" width="440" alt="NodeX Chat Demo" onerror="this.onerror=null; this.src='nodex-gui/assets/photo.jpg';" /> | <img src="docs/assets/stego_demo.gif" width="440" alt="NodeX Stego Demo" onerror="this.onerror=null; this.src='nodex-gui/assets/photo.jpg';" /> |
| *Компактные бабблы сообщений, нативный замер аудио-волны и аудиозвонки* | *Кодирование крипто-паспорта ноды в младшие биты пикселей PNG/JPG* |

---

## 🇷🇺 Описание проекта

**NodeX** — P2P-мессенджер с настольным клиентом на **Rust** и Android-клиентом на **Kotlin / Jetpack Compose**. Для обнаружения узлов используется bootstrap-узел; доставка может использовать временное DHT-хранилище, пока получатель не в сети.

Приложение спроектировано по принципу **Zero-Trust & Zero-Metadata**:
- **0 телефонных номеров, 0 Email, 0 учетных записей.**
- **Не требуется номер телефона или электронная почта.**
- Узлы находят друг друга через bootstrap и Kademlia DHT; при недоступности получателя доставка может проходить через временный mailbox в DHT.
- Сообщения и вложения шифруются сквозным шифрованием. Реальный маршрут зависит от сети, NAT и доступности узлов.

### Android-клиент

Исходники Android-приложения находятся в [`nodex-android/`](nodex-android/). Для локальной сборки нужен JDK 17 и Android SDK:

```powershell
cd nodex-android
.\gradlew.bat assembleDebug
```

APK появится в `app/build/outputs/apk/debug/app-debug.apk`.

---

## 📊 Сравнение с аналогами

| Критерий | ⚡ **NodeX** | ✈️ Telegram | 🔒 Signal | 🌀 Session | ☣️ Tox |
| :--- | :---: | :---: | :---: | :---: | :---: |
| **Центральные серверы** | ❌ **0 серверов** | ⚠️ Да | ⚠️ Да | ❌ P2P Service Nodes | ❌ 0 серверов |
| **Регистрация по номеру** | ❌ **Не требуется** | ⚠️ Обязательно | ⚠️ Обязательно | ❌ Не требуется | ❌ Не требуется |
| **Защита метаданных (IP)** | ✅ **2-Hop Onion** | ❌ Сервер видит все IP | ❌ Сервер видит все IP | ✅ Onion Routing | ⚠️ Только с Tor |
| **Стеганография (LSB)** | ✅ **Встроена** | ❌ Нет | ❌ Нет | ❌ Нет | ❌ Нет |
| **Потребление RAM** | ⚡ **~25-40 МБ** | 🐢 ~300-800 МБ | 🐢 ~400-900 МБ | 🐢 ~350-700 МБ | ⚡ ~30-60 МБ |
| **Язык разработки / Движок** | 🦀 **Pure Rust + egui** | C++ / Qt | C++ / Electron | JS / Electron | C / C++ |

---

## 🌟 Ключевые технологические фичи

### 🛡️ 1. Децентрализованная идентичность (BIP-39 & Ed25519)
Идентификатор пользователя генерируется детерминированно из 12/24 слов мнемонической фразы BIP-39. Отсутствует единая база пользователей — подделка профиля криптографически невозможна благодаря цифровой подписи Ed25519.

### 🧅 2. Слепая 2-Hop Onion маршрутизация ([onion.rs](nodex-kademlia/src/onion.rs))
При включённом onion-режиме трафик послойно зашифровывается и передается через промежуточные узлы:
```text
[Отправитель (Вы)] ──(Слой 1+2)──> [Relay Node 1] ──(Слой 2)──> [Relay Node 2] ──(Открыто)──> [Получатель]
                                (Видит ваш IP,             (Видит IP Relay 1,
                                но НЕ видит текст          но НЕ видит ваш IP)
                                 и получателя)
```

### 🖼️ 3. Стеганографические крипто-аватарки (Stego-Carriers)
Встраивание криптографического паспорта узла (`Node ID` + `X25519 PubKey` + `STUN Endpoints`) непосредственно в наименее значимые биты (LSB) пикселей любого изображения. Достаточно отправить обычную картинку в соцсети — NodeX при загрузке автоматически извлечет из нее инвайт и создаст туннель.

### 🎙️ 4. P2P Аудиозвонки & Голосовые Waveform
Прямая зашифрованная передача легковесного аудиопотока по UDP с низкой задержкой. Для голосовых сообщений генерируется вектор огибающей амплитуды для нативного отображения осциллограммы.

---

## 🏗 Архитектура системы

```text
┌─────────────────────────────────────────────────────────────────────────┐
│                    NodeX Desktop GUI (egui / eframe)                    │
│   Cyberpunk & OLED Themes • Movable Modals • Waveforms • Stego Studio   │
└────────────────────────────────────┬────────────────────────────────────┘
                                     │ UI Events / Commands (tokio mpsc)
┌────────────────────────────────────▼────────────────────────────────────┐
│                          nodex-messenger Layer                          │
│   User Identity (Ed25519 / X25519) • BIP-39 Mnemonic Seed Derivation    │
│   Stego-Carrier LSB Engine • Local Encrypted ChaCha20-Poly1305 DB       │
│   5-Tier Delivery Cascade • Audio Call Engine • Contact Trust Registry  │
└────────────────────────────────────┬────────────────────────────────────┘
                                     │ UDP Wire Packets / Kademlia RPC
┌────────────────────────────────────▼────────────────────────────────────┐
│                          nodex-kademlia Layer                           │
│   Kademlia DHT Routing • RFC 5389 STUN Traversal • UPnP IGD & NAT-PMP   │
│   UDP Hole Punching Probes • Blind 2-Hop Onion Router • UDP Chunking    │
│   Decentralized Store-and-Forward DHT Mailbox • LAN UDP Broadcast       │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## ⚡ 5-Уровневый каскад доставки (Delivery Cascade)

```mermaid
flowchart TD
    A["Попытка отправки сообщения"] --> B{"Узел получателя доступен прямо?"}
    B -- "Да" --> C["Tier 1: Direct UDP Fast-Path (0.01s)"]
    B -- "Нет" --> D{"Доступен STUN Hole Punching?"}
    D -- "Да" --> E["Tier 2: STUN Hole Punching Probe"]
    D -- "Нет" --> F{"Поддерживается UPnP / NAT-PMP?"}
    F -- "Да" --> G["Tier 3: UPnP Auto Port Mapping"]
    F -- "Нет" --> H{"Включен Onion / Relay режим?"}
    H -- "Да" --> I["Tier 4: 2-Hop Blinded Onion Relay"]
    H -- "Нет" --> J["Tier 5: Kademlia DHT Mailbox (Store & Forward)"]
```

---

## 🔒 Криптография и модель угроз

| Компонент | Алгоритм / Спецификация | Назначение |
| :--- | :--- | :--- |
| **Мастер-ключ** | `BIP-39 (12/24 words) + PBKDF2` | Детерминированная генерация сида идентичности |
| **Идентификация ноды** | `Ed25519 (RFC 8032)` | Подпись профиля, DHT-запросов и предупреждений ротации |
| **Обмен ключами** | `X25519 (ECDH Diffie-Hellman)` | Динамическое согласование сессионных ключей |
| **Сквозное шифрование** | `ChaCha20-Poly1305 AEAD` | Защита текста, файлов, аудиопотока и локальной БД |
| **Стеганография** | `LSB Matrix Encoding` | Скрытие данных внутри пикселей изображений PNG/JPG |
| **Защита от атак повтора** | `Sliding Window Nonce Cache` | Предотвращение Replay & MitM атак |

---

## 🚀 Быстрый старт

### Скачать готовый EXE (Без сборки)
👉 [**Скачать NodeX-Windows-x64.exe**](https://github.com/NetGadz/NodeX/releases/latest/download/NodeX-Windows-x64.exe)

### Сборка из исходного кода
```bash
# 1. Клонировать репозиторий
git clone https://github.com/NetGadz/NodeX.git
cd NodeX

# 2. Запустить GUI-мессенджер
cargo run -p nodex-gui --release

# 3. Запустить CLI-ноду / ретранслятор
cargo run -p nodex-cli -- --port 8443

# 4. Прогнать все тесты
cargo test --workspace
```

---

## 🇬🇧 English Overview

**NodeX** is a pure native Rust peer-to-peer messaging application engineered for zero-trust, zero-server private communication.

- **100% Serverless**: Operates purely over UDP using Kademlia DHT, RFC 5389 STUN NAT hole punching, and decentralized store-and-forward mailbox relays.
- **P2P Real-time Voice Calls**: Direct end-to-end encrypted audio calling with low latency.
- **Steganographic Crypto-Avatars**: Embed complete cryptographic network credentials inside regular PNG/JPG images.
- **Blind 2-Hop Onion Routing**: Anonymize network traffic and mask IP addresses across transit nodes.
- **Ultra-Lightweight**: Native `egui`/`eframe` GUI consuming under 40 MB RAM.
- **Direct Executable**: Download [**NodeX-Windows-x64.exe**](https://github.com/NetGadz/NodeX/releases/latest/download/NodeX-Windows-x64.exe) without needing Rust installed.

---

## 📄 Лицензия (License)

Проект распространяется под лицензией **MIT License**. Подробности в файле [LICENSE](LICENSE).

<div align="center">
  <sub>Designed & Developed with ❤️ in Rust for true digital sovereignty, privacy, and censorship resistance.</sub>
</div>
