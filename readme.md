# NodeX — P2P Messenger on Kademlia DHT

<p align="center">
  <img src="https://img.shields.io/badge/Rust-1.74%2B-orange?logo=rust" alt="Rust" />
  <img src="https://img.shields.io/badge/status-early%20prototype-yellow" alt="Status" />
  <img src="https://img.shields.io/badge/P2P-E2EE-blue" alt="P2P E2EE" />
  <img src="https://img.shields.io/badge/license-MIT-green" alt="MIT" />
</p>

<p align="center">
  <img src="https://media.giphy.com/media/26BRv0ThflsHCqDrG/giphy.gif" width="900" alt="P2P messaging preview" />
</p>

Это экспериментальный проект мессенджера, построенного поверх распределённой хеш-таблицы Kademlia. Проект находится на ранней стадии разработки: уже есть рабочая база для сетевого взаимодействия, маршрутизации, хранения данных и безопасной передачи сообщений, но архитектура, сценарии и интерфейс ещё активно меняются.

## Что это за проект

NodeX — это не “готовый коммерческий мессенджер”, а скорее продуманная исследовательская и учебная база для:

- изучения Kademlia DHT;
- реализации P2P-архитектуры в Rust;
- хранения сообщений и метаданных в распределённой сети;
- экспериментов с E2EE-подходом, идентичностью пользователя и сетевым обменом;
- дальнейшего расширения в сторону более удобного клиентского интерфейса и протоколов.

> Стадия: early prototype / pre-alpha
>
> Это хороший проект для обучения, экспериментов, улучшений, рефакторинга и совместных PR.

---

## Главные идеи

- 🌐 Децентрализованный сетевой слой на базе Kademlia
- 🔐 Шифрование и идентичность пользователя
- 💬 Сохранение сообщений и контактов локально
- 📦 Mailbox-подобный обмен через DHT для офлайн-пользователей
- 🧠 Параллельный поиск ближайших узлов и маршрутизация в сети
- 🖥️ Нативный десктопный интерфейс на egui
- 🧪 Возможность дальнейшего расширения и сопровождения

---

## Что уже есть в проекте

### P2P и маршрутизация

- `KademliaNode` с логикой маршрутизации по XOR-метрике
- `RoutingTable` и бакеты для хранения ближайших контактов
- Iterative lookup для поиска узлов и значений
- UDP транспорт в Tokio
- retry / timeout / graceful error handling

### Мессенджер

- локальная база контактов и сообщений
- E2EE-оболочка для сообщений
- офлайн-репликация через DHT Mailbox
- discovery peer по `presence_...` ключам
- простой UI с карточкой пользователя и чатом

### Система хранения и метрик

- хранение ключей и значений
- статистика RPC и трафика
- логирование сетевых событий
- поддержка сохранения состояния между запусками

---

## Демка интерфейса

<p align="center">
  <img src="https://media.giphy.com/media/l0MYt5jPR6QX5pnqM/giphy.gif" width="800" alt="NodeX demo" />
</p>

<p align="center">
  <img src="https://images.unsplash.com/photo-1516321318423-f06f85e504b3?auto=format&fit=crop&w=1200&q=80" width="800" alt="Developer workspace" />
</p>

---

## Как запустить

### 1. Сборка проекта

```bash
cargo build
```

### 2. Запуск первого узла

```bash
cargo run -- --name "Alice" --port 8000
```

### 3. Запуск второго узла с bootstrap

```bash
cargo run -- --name "Bob" --port 8001 --bootstrap 127.0.0.1:8000
```

### 4. Использование GUI

После запуска откроется нативное приложение с:

- карточкой пользователя;
- списком контактов;
- чатом;
- информацией о сетевых узлах и состоянии DHT.

---

## Пример сценария

```text
Alice -> Bob: hello from P2P network
Bob -> Alice: encrypted reply delivered via DHT mailbox
Alice -> nodes: discover peer and routing table updates
```

---

## Архитектура

```text
┌──────────────────────────────────────────────┐
│                  GUI / Desktop               │
│       (contacts, chat, node info, status)   │
└──────────────────────┬───────────────────────┘
                       │
┌──────────────────────▼───────────────────────┐
│                 KadMessenger                  │
│  - identities / presence / encrypted messages│
│  - local contact DB / message inbox          │
└──────────────────────┬───────────────────────┘
                       │
┌──────────────────────▼───────────────────────┐
│                  KademliaNode                 │
│  - routing table / lookup / UDP networking  │
│  - storage / replication / RPC               │
└──────────────────────┬───────────────────────┘
                       │
┌──────────────────────▼───────────────────────┐
│                 C FFI / crypto                │
│  - hashing / serialization / verification    │
└──────────────────────────────────────────────┘
```

---

## Roadmap

### Сейчас

- рабочий прототип сетевого модуля;
- базовый мессенджер с локальным хранилищем;
- DHT-обмен и peer discovery;
- простая GUI и демонстрационный сценарий.

### Дальше можно сделать

- [ ] более аккуратную схему обмена сообщениями;
- [ ] улучшение E2EE-модели и верификации;
- [ ] улучшенный пользовательский интерфейс;
- [ ] более стабильный offline/online flow;
- [ ] полноценную работу с контактами, профилями и группами;
- [ ] больше автоматических тестов и кейсов отказов.

---

## Структура проекта

```text
.
├── Cargo.toml
├── build.rs
├── Dockerfile
├── docker-compose.yml
├── config.example.json
├── core/
│   ├── hash.c
│   ├── hash.h
│   ├── messenger_core.c
│   ├── messenger_core.h
│   ├── serialize.c
│   └── serialize.h
├── src/
│   ├── config.rs
│   ├── crypto.rs
│   ├── db.rs
│   ├── gui.rs
│   ├── lib.rs
│   ├── lookup.rs
│   ├── main.rs
│   ├── messenger.rs
│   ├── metrics.rs
│   ├── node.rs
│   ├── rpc.rs
│   ├── state.rs
│   ├── storage.rs
│   └── ...
├── tests/
│   ├── integration_test.rs
│   └── messenger_test.rs
├── scripts/
│   ├── demo.ps1
│   └── demo.sh
├── README.md
├── CONTRIBUTING.md
├── LICENSE
└── .gitignore
```

---

## Важно

Этот проект — хороший пример “первого рабочего прототипа”: уже есть идея, сеть, логика и интерфейс, но ещё есть место для роста, оптимизаций и сильной архитектуры. Поэтому именно сейчас особенно полезны идеи, правки, рефакторинг и предложения по улучшению.

Если хочется, можно развивать проект в сторону:

- более безопасной P2P-мессенджер-архитектуры;
- живой CLI/GUI и панели статуса;
- лучшей синхронизации контактов и сообщений;
- более понятного API и расширяемых модулей.

---

## Contributing

В проекте всё ещё много пространства для улучшений, и любые практические идеи очень приветствуются. Подробнее — в [CONTRIBUTING.md](CONTRIBUTING.md).

<p align="center">
  <img src="https://media.giphy.com/media/3o7aD2saalBwwftBIY/giphy.gif" width="700" alt="Collaboration" />
</p>

---

## License

MIT