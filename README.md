# AIL Runtime

**Semantic Runtime for Code and Content**

AIL Runtime — это исполняемый Semantic Layer, в котором:

- Код представлен как **Binary AST** (граф операций)
- Работает **Ownership**-модель
- Поддерживается безопасный **Hot-Swap** с lineage
- Код и контент рассматриваются как две стороны одного графа
- Используется визуальный язык рёбер **Star Gate**
- Структура доменов опирается на **GERMES**-гексагон

## Текущий статус

- [x] Binary AST + интерпретатор (MVP)
- [x] Базовый Ownership
- [x] Hot-Swap + Lineage
- [x] Правила компиляции `.ail` → Binary AST
- [x] Star Gate (визуальный язык рёбер)
- [x] GERMES-гексагон как топология доменов
- [x] Реализация v2 (`mvp/src/v2/`): канонический алфавит, вычисляемый GERMES, контракты и refinement, ownership-анализ, выводимый граф связей, Merkle-lineage, метрики здоровья
- [ ] Полный компилятор `.ail`
- [ ] VizGraph (визуализация)
- [ ] M2M-сериализация (парсер и сериализатор M2M-строк добавлены в v2)

## Быстрый старт

```bash
git clone https://github.com/svend4/ail-runtime.git
cd ail-runtime
cd mvp
cargo test
```

## Структура

```text
ail-runtime/
├── .github/workflows/ci.yml  # CI-проверка сборки и тестов
├── README.md
├── LICENSE                   # GNU Affero General Public License v3 (AGPL-3.0)
├── docs/
│   ├── specification.md      # Полная спецификация v0.1
│   ├── visual-edges.md       # Star Gate — язык рёбер
│   ├── architecture.md       # Архитектура
│   ├── audit/                # Концептуальный, технический и практический аудит (октябрь 2026)
│   └── v2/                   # Спецификация v0.2
├── mvp/                      # Рабочий прототип на Rust (v1 + параллельный вариант v2)
│   └── src/
│       └── v2/               # 12 модулей v2 (alphabet, germes, ast/contract, ownership, relation, health, lineage, actor)
├── tools/                    # Виртуальная клавиатура Star Gate и заметки по шрифтам Solan
└── examples/
```

## Visual Edges (Star Gate)

| Символ | Значение                     | Применение                     |
|--------|------------------------------|--------------------------------|
| `—`    | Обычная связь                | Control / Data                 |
| `→`    | Направленный поток           | Data-flow                      |
| `△`    | Влияние / порождение         | Создание значения              |
| `◇`    | Устойчивая связь             | Долгоживущие отношения         |
| `□̸`   | Shared borrow                | `Borrowed`                     |
| `▲`    | Unique borrow                | `BorrowedMut`                  |
| `✕`    | Конфликт ownership           | Конфликт `&` + `&mut`          |
| `⬡`    | Гиперребро / группа          | AtomicUpdate, OwnershipGroup   |

## Философия

> Continuum стабилизирует **форму**.  
> AIL Runtime стабилизирует **смысл**.

## Аудит и вариант v2 (ветка `claude/busy-pasteur-xxbvai`)

В этой ветке проведён технический, концептуальный и практический аудит проекта и добавлен параллельный вариант ядра **v2** (12 модулей, 55 тестов, clippy-чистая сборка) — без удаления или переписывания существующего кода и документов (в `mvp/src/lib.rs` добавлено `extern crate self as ail_mvp;` для сборки библиотеки v1).

- Сводка и навигация: [`docs/audit/README.md`](docs/audit/README.md)
- Реестр замечаний (35 пунктов): [`docs/audit/05-findings-register.md`](docs/audit/05-findings-register.md)
- Собственный вариант и дорожная карта: [`docs/audit/04-v2-proposal.md`](docs/audit/04-v2-proposal.md)
- Спецификация v0.2: [`docs/v2/specification-v0.2.md`](docs/v2/specification-v0.2.md)
- Код: [`mvp/src/v2/`](mvp/src/v2/)

```bash
cd mvp
cargo test                 # 55 тестов v2 + 1 тест v1
cargo run --bin ail-mvp    # демо v1
cargo run --bin v2-demo    # демо v2: выведенный граф, контракты, ownership, lineage
cargo clippy --lib --bins  # проверка без предупреждений
```

## Лицензия

Проект распространяется под лицензией **GNU Affero General Public License v3.0 (AGPL-3.0)** — см. [`LICENSE`](LICENSE) (синхронизировано с веткой `main`; в ранних заметках упоминалась лицензия MIT).
