# Технический аудит кода `svend4/ail-runtime` (Wave 2)

- **Ветка аудита:** `audit/code-review` (создана от `consolidate/ready-for-main`, базовый коммит `d74254ca5a19bbb1ad53177c257bb09d5776aa26`)
- **Проверенные файлы и директории:** `mvp/Cargo.toml`, `mvp/Cargo.lock`, `mvp/src/lib.rs`, `mvp/src/main.rs`, `mvp/src/connectivity.rs`, `mvp/src/vizgraph.rs`, `mvp/src/lever.rs`, `mvp/src/integration_demo.rs`, `mvp/src/actor_with_graph.rs`, `mvp/src/viz_demo.rs`, `mvp/src/bin/v2-demo.rs`, `mvp/src/v2/` (`mod.rs`, `alphabet.rs`, `germes.rs`, `ast.rs`, `symbolic.rs`, `contract.rs`, `derive.rs`, `relation.rs`, `health.rs`, `lineage.rs`, `ownership.rs`, `vm.rs`, `actor.rs`), `.github/workflows/ci.yml`, `README.md`, `docs/audit/README.md`.

---

## 1. Структура крейта и организация модулей

В корне репозитория `Cargo.toml` отсутствует; единый Rust-пакет расположен в подкаталоге `mvp/`:

- **Манифест (`mvp/Cargo.toml`)**: пакет `ail-mvp` версии `0.1.0`, `edition = "2021"`, lockfile `mvp/Cargo.lock` зафиксирован.
- **Зависимости**: `serde` (`1.0`, `features = ["derive"]`), `serde_json` (`1.0`), `sha2` (`0.10`). Внешних зависимостей для графов, логирования или асинхронности нет — ядро полностью self-contained.
- **Таргеты**:
  - библиотека `ail_mvp` (`mvp/src/lib.rs`),
  - основной бинарник `ail-mvp` (`mvp/src/main.rs`),
  - бинарник демонстрации v2 `v2-demo` (`mvp/src/bin/v2-demo.rs`).
- **Сборка v1 внутри крейта (`mvp/src/lib.rs`)**: используется аддитивный обходной приём `extern crate self as ail_mvp;`, благодаря которому замороженные модули `vizgraph.rs` и `lever.rs` (а также `integration_demo.rs` в `main.rs`) разрешают пути `ail_mvp::connectivity::*` без модификации исходников v1.
- **Неподключённый legacy-код в `mvp/src/`**: файлы `actor_with_graph.rs` и `viz_demo.rs` присутствуют в дереве исходников, но не объявлены ни в `lib.rs`, ни в `main.rs` (`viz_demo.rs` импортирует `crate::actor_with_graph`, ожидая подключения через `main.rs`), поэтому они не компилируются при `cargo build` / `cargo test`.

---

## 2. Сравнительный анализ v1 и v2: безопасность, ownership и контракты

### 2.1. Уровень безопасности и обработки ошибок

| Аспект | v1 (`main.rs`, `connectivity.rs`, `vizgraph.rs`, `lever.rs`) | v2 (`mvp/src/v2/`) |
|---|---|---|
| Возврат ошибок | Частые `.expect("node must exist")` и `.unwrap()` в `Graph::neighbors` / `Graph::add_edge` (`connectivity.rs`) | Явные типы ошибок и `Result`: `RelationError`, `VmError`, `LineageError`, `SwapReport` |
| Побочный вывод в библиотеке | Вызовы `println!` в `ShardActor::hot_swap`, `print_lineage` (`main.rs`), `VizGraph::print_scene` (`vizgraph.rs`), `LeverBalance::print` (`lever.rs`) | Логика возвращает значения/отчёты (`render()` -> `String`), вывод вынесен в бинарник `mvp/src/bin/v2-demo.rs` |
| Ограничение исполнения | Нет лимита шагов или глубины вызовов | `Vm` ограничивает исполнение через `fuel` (`1_000_000`) и `max_depth` (`64`), проверяет границы регистров и целочисленное переполнение (`checked_add`/`checked_sub`) |

Наблюдения по остаточным инвариантам и граничным случаям в v2:
1. В v2 сохраняются точечные `.expect(...)` / `.unwrap()` / прямое индексирование для внутренних структурных инвариантов: `Symbol::spec().expect(...)` (`alphabet.rs`), `ShardActor::health()` и `ShardActor::swap()` (`actor.rs`), `Lineage::append().unwrap()` и `serde_json::to_vec` (`lineage.rs`), `self.nodes[n]` / `self.edges[e.0]` (`relation.rs`), `fn_nodes[...]` (`derive.rs`), `Q3::parse(...).unwrap()` и `partial_cmp(...).unwrap()` (`germes.rs`). В рамках корректно построенных объектов это инварианты, однако внешним потребителям стоит это учитывать.
2. В `Lineage::render` (`lineage.rs`) используется прямой байтовый срез `&g.id[..12]` и `&g.content_hash[..12]`. Это безопасно для 64-символьных hex-хэшей SHA-256, но вызовет панику при передаче десериализованного объекта с более коротким `id`/`content_hash`.

### 2.2. Ownership-модель

- **v1**: В `mvp/src/main.rs` состояние регистра `Value::Consumed` проверяется только динамически при исполнении (`ExecError::UseAfterConsume`). Статического анализатора нет; передача одного и того же `ObjectRef` в два мутируемых параметра (`Transfer(w, w, 300)`) допускается без ошибок.
- **v2 (`ownership.rs`, `vm.rs`)**:
  - `ownership::analyze(f: &Function)` статически проверяет: выход за пределы `f.regs` (`InvalidReg`), чтение неинициализированного регистра (`UseBeforeInit`), использование после `Move`/`Consume` (`UseAfterConsume`, `DoubleConsume`), возврат потреблённого значения (`ReturnConsumed`), отсутствие `Return` (`MissingReturn`), код после `Return` (`UnreachableAfterReturn`), а также фиксирует `dead_stores`.
  - Для каждой пары параметров одного объектного типа `ParamType::Object(TypeTag)`, оба из которых попадают в `eff.writes`, формируется обязательство `Obligation::DistinctMutableParams { a, b }`.
  - `Vm::with_heap` предвычисляет эти обязательства и при вызове (`Vm::check_args`) отклоняет алиасинг с `VmError::AliasedUniqueParams`, наряду с проверкой арности (`ArityMismatch`) и типов параметров (`ParamTypeMismatch`).
  - **Выявленный пробел Wave 2**: `Vm::with_heap` (`vm.rs`) извлекает из `analyze(f)` только `.obligations`, игнорируя `analyze(f).errors`. Если `Vm` создаётся и вызывается напрямую в обход `ShardActor::swap`, функция со статической ошибкой ownership (например, `MissingReturn` или потенциальным `UseBeforeInit`) может быть допущена к исполнению вплоть до runtime-ошибки. Аналогично `ShardActor::with_policy` (`actor.rs`) при инициализации genesis-модуля вычисляет граф и lineage, но не прогоняет `ownership::analyze`.
  - **Межпроцедурный эффект вызовов**: в `symbolic::effects` (`symbolic.rs`) операция `Op::Call` добавляет имя функции в `eff.calls` и связывает регистр-приёмник с `Sym::Call(func, args)`, а `effects_in` инлайнит только *чистые* (`e.is_pure()`) функции на глубину `DEPTH = 8`. Побочные эффекты записи *нечистых* вызываемых функций не распространяются на вызывающую функцию и не порождают дополнительных `DistinctMutableParams`-обязательств для вызывающего кода.

### 2.3. Контракты и hot-swap

- **v1 (`main.rs`, `connectivity.rs`)**:
  - `ShardActor::hot_swap` отклоняет обновление только при удалении имени функции (`!candidate.functions.iter().any(|f| f.name == old_fn.name)`). Сигнатуры, guard'ы и побочные эффекты не проверяются.
  - `compute_content_hash` хэширует только `name` и `functions`, поэтому повторный деплой даёт одинаковый хэш в разных поколениях.
  - В `connectivity.rs` формула `saturation` включает ребро `(u, v)` в знаменатель (`deg_u_w + deg_v_w`), из-за чего `reinforce_edge` снижает насыщенность; `can_borrow_unique` разрешает `D2`, вопреки наличию `BorrowError::UniqueNotAllowedOnD2`.
- **v2 (`symbolic.rs`, `contract.rs`, `derive.rs`, `relation.rs`, `lineage.rs`, `actor.rs`)**:
  - Символьное исполнение (`effects`, `effects_in`) абстрагируется от конкретных регистров (register-independent `Sym`) и инлайнит чистые вызовы (например, делегирование проверки в `CheckBalance`).
  - `contract::check_refinement` строго разделяет нарушения (`Violation`: `Removed`, `ArityChanged`, `ParamTypeChanged`, `GuardDropped`, `NewWrite`) и допустимые изменения (`Change`: `Added`, `NewRead`, `GuardAdded`, `WriteChanged`, `ReturnChanged`, `CallsChanged`) с классификацией `Change::is_semantic()`. (Замечание: при `ArityChanged` сравнение для этой функции завершается через `continue`, не собирая сопутствующие различия.)
  - `derive::derive_with` выводит рёбра (`Conflict` `✕`, `Unique` `▲`, `Shared` `□̸`, `Directed` `→`, `HyperMember` `⬡`) непосредственно из `Effects`, а `relation::RelationGraph` вычисляет монотонную трёхкомпонентную насыщенность (топология × поддержка, usage/pheromone, symbol prior).
  - `actor::ShardActor::propose` объединяет проверку контрактов, статический `ownership::analyze` и оценку риска по рёбрам в единый `SwapReport`, а `commit` обновляет `Lineage` с раздельными `content_hash` и `chain_id`.

---

## 3. Анализ CI/CD (`.github/workflows/ci.yml`)

На момент аудита в `.github/workflows/ci.yml` настроено:
- **Триггеры**: `push: branches: ["**"]` и `pull_request:` (покрывают все ветки и PR).
- **Среда**: `ubuntu-latest`, `working-directory: mvp`, `dtolnay/rust-toolchain@stable` (`clippy`, `rustfmt`), `Swatinem/rust-cache@v2`.
- **Проверки**:
  1. `cargo build --all-targets --locked`
  2. `cargo test --locked`
  3. `cargo clippy --lib --bins --locked -- -D warnings -A clippy::all -W clippy::correctness`
  4. `rustfmt --edition 2021 --check src/v2/*.rs src/bin/*.rs`
  5. Запуск демо `cargo run --quiet --bin ail-mvp` и `cargo run --quiet --bin v2-demo`.

**Выявленные недостатки CI:**
1. **Ослабленный профиль Clippy**: флаги `-A clippy::all -W clippy::correctness` отключают все стандартные линты (`style`, `perf`, `complexity`, `suspicious`) и проверяют только `--lib --bins` (без `--tests`). Причина понятна (замороженный legacy-слой v1 в том же крейте), но из-за этого фактическая чистота `v2` по полному набору линтов Clippy в CI не контролируется.
2. **Неполный охват `rustfmt`**: проверяются только `src/v2/*.rs` и `src/bin/*.rs`, тогда как `src/lib.rs` в проверку не включён.
3. **Отсутствие hardening-параметров и неполный `--locked`**: в `.github/workflows/ci.yml` не заданы `permissions: contents: read` и `concurrency` (`cancel-in-progress: true`), а в шаге `Run demos` команды `cargo run` не передают `--locked`.

*(Примечание: модификация `.github/workflows/ci.yml` не применялась из данной автоматизации, так как используемый GitHub PAT не имеет scope `workflow` — `POST /git/trees` вернул `403 Resource not accessible by personal access token`. Рекомендуемый патч приведён ниже.)*

Рекомендуемый безопасный патч для `.github/workflows/ci.yml`:
```yaml
permissions:
  contents: read

concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: true
```
и в шаге `Run demos`:
```bash
cargo run --quiet --locked --bin ail-mvp
cargo run --quiet --locked --bin v2-demo
```

---

## 4. Изменения, внесённые в ветке `audit/code-review`

1. **`mvp/src/lib.rs`**: расширена крейтовая документация (`//!`) с явным разграничением замороженного legacy-слоя v1 и модулей аудированного ядра v2 (безопасное неломающее улучшение документации без изменения поведения).
2. **`docs/AUDIT_REPORT.md`**: добавлен настоящий отчёт технического аудита Wave 2.

---

## 5. Рекомендуемые следующие инженерные шаги

1. **Усилить `.github/workflows/ci.yml`** с правами `workflow`: добавить `permissions: contents: read`, `concurrency` (`cancel-in-progress: true`) и `--locked` к командам `cargo run` в шаге `Run demos`.
2. **Разделить крейты v1 и v2 через Cargo workspace**: выделить `v2` в отдельный крейт (или workspace member), чтобы включить строгий `cargo clippy --all-targets -- -D warnings` и полный `cargo fmt --check` для нового кода, не затрагивая замороженный v1.
3. **Включить проверку `ownership::analyze` на входе в `Vm` и `ShardActor::with_policy`**: добавить `Vm::try_new` (или проверять `analyze(f).is_ok()` в `ShardActor::with_policy` и `Vm::with_heap`), чтобы некорректные по ownership функции нельзя было исполнить напрямую в обход `ShardActor::swap`.
4. **Распространить эффекты нечистых вызовов (`Op::Call`)**: учитывать `typed_writes` / `typed_reads` вызываемых функций в `symbolic::effects_in` и проверять алиасинг аргументов при вызове нечистых функций.
5. **Защитить `Lineage::render` и навести порядок в мёртвых файлах**: заменить `&g.id[..12]` / `&g.content_hash[..12]` на безопасный срез (`get(..12).unwrap_or(...)`), а неиспользуемые `actor_with_graph.rs` и `viz_demo.rs` явно задокументировать как архивные шаги либо удалить в отдельной cleanup-задаче.
