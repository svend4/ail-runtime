# 02 — Технический аудит кода

Все утверждения ниже воспроизводимы на коммите `0e84332` (HEAD ветки `main` на момент аудита) плюс минимальный аддитивный фикс сборки из этой ветки. Среда: Rust 1.97.0, Linux.

## T1. Библиотека не собирается

```text
$ cd mvp && cargo build
error[E0433]: cannot find module or crate `ail_mvp` in this scope
 --> src/vizgraph.rs:7:5
error[E0433]: cannot find module or crate `ail_mvp` in this scope
 --> src/lever.rs:3:5
… error: could not compile `ail-mvp` (lib) due to 13 previous errors
```

**Причина.** `vizgraph.rs` и `lever.rs` входят в библиотеку (`lib.rs`: `pub mod vizgraph; pub mod lever;`), но импортируют `use ail_mvp::connectivity::*;` — крейт нельзя назвать по собственному имени изнутри без `extern crate self as …`.

**Фикс в ветке (аддитивный):** в конец `lib.rs` добавлена строка `extern crate self as ail_mvp;`. Канонический фикс — заменить на `use crate::connectivity::*;` (требует правки существующих файлов; оставлено владельцу).

**Серьёзность:** блокирующая. Означает, что после коммита `d80e112` (Step 5) ни `cargo build`, ни `cargo test` ни разу не выполнялись успешно.

## T2. Модули «шагов 1–5» не подключены и не компилируются

`main.rs` объявляет только `mod integration_demo;`. Файлы `actor_with_graph.rs` (Steps 1–4) и `viz_demo.rs` (Step 5) не объявлены ни в `main.rs`, ни в `lib.rs` — Cargo их не компилирует. Коммит `9a6541a` «Wire VizGraph demo into actor steps» добавил `viz_demo.rs`, но не изменил `main.rs`.

При подключении через `#[path]` в отдельный бинарник:

```text
error[E0277]: the trait bound `ail_mvp::SwapRisk: Clone` is not satisfied
  --> actor_with_graph.rs:37:5   (derive(Clone) на RuntimeMetrics с полем Option<SwapRisk>)
error[E0502]: cannot borrow `*self` as mutable because it is also borrowed as immutable
  --> actor_with_graph.rs:98:23  (rebuild_relation_graph: alloc_node() внутри for по &self.module.functions)
```

**Вывод:** записи «✅» для шагов 1–5 в `IMPLEMENTATION_STATUS.md` и `step5-vizgraph.md` описывают код, который никогда не был скомпилирован. Предлагаемый патч (2 строки): `#[derive(Clone, Copy, …)]` для `SwapRisk`; в `rebuild_relation_graph` сначала собрать `let names: Vec<String> = self.module.functions.iter().map(|f| f.name.clone()).collect();`.

## T3. Два несовместимых определения модуля

| Тип | `main.rs` | `actor_with_graph.rs` |
|-----|-----------|------------------------|
| `BinaryFunction` | `name, params: u8, body: Vec<Op>` | `name` |
| `BinaryModule` | с `content_hash` по SHA-256 | с `content_hash = "hash-gen-N"` |
| `ModuleOrigin` | `Manual / Healed / Loaded` | `Manual / Healed` |

Граф связей (библиотека) не имеет доступа к `Op` (бинарник). Отсюда ручные `link_functions`.

## T4. Насыщенность анти-монотонна по весу

`connectivity.rs::saturation`: `s = 2·common_weight / (wdeg(u) + wdeg(v))`, где `wdeg` включает вес ребра `(u,v)`, а `common_weight` — нет.

Проверка (треугольник, веса 1.0, затем `reinforce_edge(a,b,+5.0)`):

```text
triangle sat(a,b) before reinforce      = 0.500 (D2)
triangle sat(a,b) after  reinforce(+5)  = 0.143 (D1)
```

Единственный тест проекта (`weighted_saturation_increases_with_weight`) проверяет лишь принадлежность `[0,1]` и потому не ловит инверсию.

## T5. D3 недостижим; демо противоречит своим ожиданиям

Невзвешенная формула для клики Kₙ: `s = (n−2)/(n−1)`.

```text
K3 sat=0.500 D2    K4 sat=0.667 D2    K5 sat=0.750 D2
K6 sat=0.800 D3    K7 sat=0.833 D3    K8 sat=0.857 D3
```

Вывод `cargo run --bin ail-mvp` (после фикса T1):

```text
=== Relation Graph Report ===
  NodeId(1) -> NodeId(3) | symbol=Directed | sat=0.000 | degree=D1
  NodeId(3) -> NodeId(4) | symbol=Simple   | sat=0.000 | degree=D1
  NodeId(2) -> NodeId(4) | symbol=Unique   | sat=0.000 | degree=D1
  NodeId(1) -> NodeId(2) | symbol=Conflict | sat=0.000 | degree=D1
  marginality_ratio = 0.333

1. Attempt to downgrade Transfer ✕ Refund (expect Critical/High):
Hot-swap risk for NodeId(1) -> NodeId(2): Low
   Allowed
```

Все четыре ребра — D1, конфликт — `Low / Allowed`.

## T6. Риск связности в `hot_swap` не может сработать

`actor_with_graph.rs::hot_swap`: (1) проверка имён отклоняет любое удаление; (2) `estimate_swap_risk` считает ребро затронутым только если один из его концов *отсутствует* в новом списке имён. После (1) это невозможно ⇒ риск всегда `Low`. Шаг 2 («hot-swap consults connectivity risk») функционально пуст.

## T7. Три разные матрицы риска

| old → new | `hot_swap_risk` (код) | `estimate_swap_risk` (код) | `saturation-usage.md` §5 |
|-----------|----------------------|----------------------------|--------------------------|
| D1 → D2 | Low | — | Low–Medium |
| D2 → D2 | Medium | High (любое D2) | Medium |
| D3 → D1 | Critical | Critical | **High** |

## T8. Паники и некорректные состояния в публичном API

- `Graph::neighbors` — `expect("node must exist")`: паника на неизвестном `NodeId` из любого вызова `saturation`.
- `add_edge` одной и той же пары дважды создаёт два `Edge` (проверено: `edges.len()==2`, веса `[1.0, 2.0]`), при одном ключе веса — `Edge.weight` рассинхронизирован.
- `reinforce_edge` на несуществующем ребре молча создаёт вес без ребра.
- `BorrowError::ConflictOnD3`, `UniqueNotAllowedOnD2` никогда не конструируются; `can_borrow_shared` всегда `true`.

## T9. Направленность теряется

`adj` симметрична; `StarGateSymbol::Directed` не влияет ни на насыщенность, ни на риск. `from/to` — только метка.

## T10. Интерпретатор v1

- `params: u8` не сверяется с числом аргументов; лишние/недостающие аргументы молча остаются в регистрах.
- Нет `Call`, ветвлений, циклов — межфункциональные рёбра невозможно вывести.
- `Return { value: None }` возвращает `Bool(true)` (не `Unit`).
- Фиксированные 16 регистров на актор; регистры не очищаются между вызовами — утечка состояния между вызовами.
- `ObjectRef` копируется; `Move` защищает регистр, но не объект; алиасинг параметров не проверяется (см. 01 §3.2, воспроизведено в `v2::vm::tests::without_the_check_the_documented_order_mints_money`).
- `println!` внутри `hot_swap` — побочный эффект в логике библиотеки.

## T11. Lineage

Вывод демо v1:

```text
gen  0 | v1   | e55418a48857 | Manual
gen  1 | v2   | e55418a48857 | Healed from gen 0 (compatible update)
```

Один и тот же хэш у двух поколений; `parent_hash` указывает на идентичную строку. Хэш считается через `serde_json::json!({...}).to_string()` — зависит от текстового представления `Debug`-подобного JSON. Нет функции верификации цепи.

## T12. Тесты, CI, гигиена

| Аспект | Состояние |
|--------|-----------|
| Тесты | 1 (`connectivity::tests`), ассерты `0 ≤ s ≤ 1` |
| CI | отсутствует |
| `.gitignore` | отсутствует (`target/` не исключён) |
| `Cargo.lock` | не в репозитории (для бинарного крейта — рекомендован) |
| `LICENSE` | отсутствует (README декларирует MIT) |
| Форматирование | исходники v1 не проходят `rustfmt --check` |
| clippy | предупреждения только в тесте v1 |

## T13. Расхождение документов и кода

- `README.md`: «VizGraph — [ ]», «M2M-сериализация — [ ]»; `IMPLEMENTATION_STATUS.md`: VizGraph ✅. Структура в README не упоминает `tools/`, `docs/steps/`.
- `specification.md` помечена v0.1, не версионируется при изменениях.
- `connectivity-degree.md` даёт две разные взвешенные формулы (через `sqrt(deg·deg)` и через среднее), код реализует третью.

## T14. Инструменты и шрифты

- `tools/fonts/*.ttf.b64` — файлы из 11 байт с текстом `placeholder`; коммит `73d646d` называет их «base64 sidecars».
- `install-fonts.sh` не декодирует base64, только проверяет наличие `.ttf`.
- `stargate-calculator.html` ссылается на `fonts/myFront4Solan4.ttf`, которых в репозитории нет.
- Макросы (`Lnk`, `Bal`, `Rei`, `RAG`…) вставляют текст в `textarea`; ничто его не разбирает и не исполняет.
- Лицензия и происхождение шрифтов `myFront4Solan3/4` не указаны.

## T15. Безопасность и устойчивость (заметки на будущее)

Текущий код не сетевой, поэтому поверхность атаки минимальна. Но концепция (hot-swap модулей, M2M-gossip, lineage) требует:

- подписи поколений (кто и чем подписал swap);
- лимитов исполнения (fuel, глубина вызовов) — в v2 добавлены;
- проверки типов аргументов на границе актора — в v2 добавлена;
- защиты от самоссылок/дубликатов в графе — в v2 `Result` вместо паник.

## T16. Сводная таблица объёма

| Компонент | Файлы | Статус до ветки | Статус в ветке |
|-----------|-------|-----------------|----------------|
| v1 lib (`connectivity`, `vizgraph`, `lever`) | 3 | не собирается | собирается (аддитивный фикс) |
| v1 bin (`main`, `integration_demo`) | 2 | не собирается (из-за lib) | работает |
| v1 dormant (`actor_with_graph`, `viz_demo`) | 2 | не компилируются | без изменений (замечание T2, патч предложен) |
| v2 lib (`v2/*`) | 12 | — | 55 тестов, clippy-чисто, rustfmt-чисто |
| v2 bin (`v2-demo`) | 1 | — | работает |
| Тесты | 1 | 1 | 56 |
| CI | — | нет | `.github/workflows/ci.yml` |
