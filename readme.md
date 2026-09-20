<div align="center">

<img src="nodex-gui/assets/photo.jpg" width="160" alt="NodeX Logo" style="border-radius: 20px; margin-bottom: 12px;" />

# ⚡ NodeX

**Децентрализованный P2P E2EE Мессенджер нового поколения на чистом Rust**
<br />
*Zero-Server • 100% Serverless • STUN NAT Hole Punching • Kademlia DHT • 2-Hop Onion Routing • Stego-Carriers*

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg?style=for-the-badge)](https://www.gnu.org/licenses/gpl-3.0)
[![Rust 2021](https://img.shields.io/badge/Rust-2021-orange.svg?style=for-the-badge&logo=rust)](https://www.rust-lang.org/)
[![GUI](https://img.shields.io/badge/GUI-egui%20%2F%20eframe-22c55e.svg?style=for-the-badge)](https://github.com/emilk/egui)
[![Security](https://img.shields.io/badge/Crypto-ChaCha20--Poly1305-8b5cf6.svg?style=for-the-badge)](https://github.com/RustCrypto)
[![Network](https://img.shields.io/badge/P2P-Kademlia%20DHT-0ea5e9.svg?style=for-the-badge)](https://en.wikipedia.org/wiki/Kademlia)

[English](#-english-overview) • [Русский](#-описание-проекта) • [Архитектура](#-архитектура) • [Быстрый старт](#-быстрый-старт) • [Безопасность](#-криптография-и-безопасность)

</div>

---

## 🇷🇺 Описание проекта

**NodeX** — это автономный кроссплатформенный мессенджер с открытым исходным кодом, работающий полностью без центральных серверов, телефонных номеров и учетных записей. Все коммуникации (текст, файлы, голосовые сообщения и прямые аудиозвонки) происходят напрямую между узлами пользователей с использованием сквозного шифрования (E2EE), распределенной хеш-таблицы (Kademlia DHT) и автоматического пробития NAT-роутеров.

---

## 🌟 Ключевые возможности

- 🛡️ **Полная автономность и анонимность**: 0 номеров телефона, 0 email, 0 центральных серверов. Идентификация по криптографическому ключу Ed25519/X25519, генерируемому из 24 слов сид-фразы BIP-39.
- 📞 **P2P Голосовые звонки в реальном времени**: Прямой легковесный зашифрованный UDP-стрим низкой задержки (PCM 16-bit / 44.1kHz).
- 🎙️ **Голосовые сообщения с реальной Waveform**: Запись микрофона, мгновенная генерация амплитудной огибающей и воспроизведение в один клик.
- 🖼️ **Стеганографические крипто-аватарки (Stego-Carriers)**: Встраивание зашифрованного цифрового паспорта ноды в младшие биты пикселей (LSB) любого изображения (PNG/JPG). Достаточно отправить картинку другу — NodeX сам извлечет контакт и откроет туннель.
- 🧅 **Слепая 2-Hop Onion маршрутизация**: Сокрытие реального IP-адреса отправителя за двумя промежуточными случайными шифрованными ретрансляторами в сети.
- 📱 **QR-спаривание устройств и мульти-сессии**: Быстрое подключение мобильных и десктопных компаньонов через одноразовые криптографические QR-токены (`nodex://pair/...`).
- 🔗 **Персональные Invite-ссылки (`nodex://invite/...`)**: Мгновенный обмен контактами с автоматической передачей публичных ключей и STUN-координат.
- 🎨 **Современный нативный GUI**:
  - Свободно перемещаемые и масштабируемые окна (Movable & Resizable Modals).
  - Динамические темы оформления: **Dark**, **Midnight (OLED Pure Black)** и **Day Light**.
  - Компактный и продуманный вид сообщений без растягивания на весь экран.
  - Локальный PIN-код и защита от подглядывания.

---

## 🏗 Архитектура системы

```
┌────────────────────────────────────────────────────────────────────────┐
│                     NodeX Desktop GUI (egui / eframe)                  │
│   Themes • Compact Bubbles • Movable Windows • Voice Waveforms • QR    │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │ Events / UI Commands
┌───────────────────────────────────▼────────────────────────────────────┐
│                         nodex-messenger Layer                          │
│   BIP-39 Mnemonic • User Identity (Ed25519 / X25519) • PFS Sessions    │
│   Stego-Carrier LSB • Signed Invites • Local ChaCha20-Poly1305 DB      │
│   5-Tier Delivery Cascade • Audio Call Stream • P2P Groups • Contacts  │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │ Kademlia RPC & Transports
┌───────────────────────────────────▼────────────────────────────────────┐
│                         nodex-kademlia Layer                           │
│   Kademlia DHT Routing • RFC 5389 STUN Traversal • UPnP Port Forward   │
│   Direct UDP Hole Punching • 2-Hop Onion IP Masking • Packet Chunking  │
│   Encrypted Store-and-Forward DHT Mailbox • Multi-Source Bootstrap     │
└────────────────────────────────────────────────────────────────────────┘
```

---

## ⚡ 5-Уровневый каскад доставки (Delivery Cascade)

1. **Tier 1: Прямой UDP Fast-Path** — моментальная доставка, если узел получателя доступен напрямую.
2. **Tier 2: STUN Hole Punching** — автоматическое пробитие Cone NAT и домашних роутеров через `stun.l.google.com:19302` и `stun.cloudflare.com:3478`.
3. **Tier 3: UPnP Automatic Port Mapping** — автоматический проброс портов на совместимых роутерах.
4. **Tier 4: 2-Hop Onion Relay** — передача через зашифрованные промежуточные ноды со скрытием IP.
5. **Tier 5: Децентрализованный DHT Mailbox** — безопасное хранение зашифрованных сообщений в DHT-сети, если получатель оффлайн.

---

## 🚀 Быстрый старт

### Требования
- Установленный [Rust](https://rustup.rs/) (версия 1.75+).

### 1. Запуск десктопного мессенджера (GUI)
```bash
cargo run -p nodex-gui
```

### 2. Запуск фоновой ноды / ретранслятора (CLI)
```bash
cargo run -p nodex-cli -- --port 8443
```

### 3. Запуск полного набора тестов
```bash
cargo test --workspace
```

---

## 🔒 Криптография и безопасность

| Компонент | Алгоритм / Стандарт | Назначение |
| :--- | :--- | :--- |
| **Идентификация** | `Ed25519 (RFC 8032)` | Цифровая подпись профиля, присутствия и сообщений |
| **Сквозное шифрование (E2EE)** | `X25519 + ChaCha20-Poly1305` | Шифрование текста, аудио, файлов и звонков |
| **Сид-фраза восстановления** | `BIP-39 (24 слова)` | Детерминированное восстановление учетной записи |
| **Локальная БД** | `ChaCha20-Poly1305` | Зашифрованное хранение переписки и контактов на диске |
| **Стеганография** | `LSB Matrix Steganography` | Сокрытие криптографических данных внутри изображений |
| **Защита от повторов** | `Nonce Cache + Timestamp Window` | Защита от Replay и MitM атак |

---

## 🇬🇧 English Overview

**NodeX** is a modern, pure-native Rust peer-to-peer messaging application engineered for zero-trust, zero-server private communication.

- **100% Serverless**: Operates purely over UDP using Kademlia DHT, Google/Cloudflare STUN NAT hole punching, and decentralized mailbox relays.
- **P2P Real-time Voice Calls**: Direct end-to-end encrypted audio calling with low latency.
- **Steganographic Crypto-Avatars**: Embed complete cryptographic network credentials inside regular PNG/JPG images.
- **Blind 2-Hop Onion Routing**: Anonymize network traffic and mask IP addresses across transit nodes.
- **Movable & Resizable UI**: Built with `egui` and `eframe` with Dark, Midnight OLED, and Day themes.

---

## 📄 Лицензия (License)

Проект распространяется под лицензией **GNU General Public License v3.0 (GPL-3.0)**.
Подробности доступны в файле [LICENSE](LICENSE).

<div align="center">
  <sub>Built with ❤️ and Rust for privacy, freedom, and true decentralization.</sub>
</div>
