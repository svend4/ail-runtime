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
- [ ] Полный компилятор `.ail`
- [ ] VizGraph (визуализация)
- [ ] M2M-сериализация

## Быстрый старт

```bash
git clone https://github.com/svend4/ail-runtime.git
cd ail-runtime
```

## Структура

```text
ail-runtime/
├── README.md
├── docs/
│   ├── specification.md      # Полная спецификация v0.1
│   ├── visual-edges.md       # Star Gate — язык рёбер
│   └── architecture.md       # Архитектура
├── mvp/                    # Рабочий прототип
│   └── src/
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

## Лицензия

MIT
