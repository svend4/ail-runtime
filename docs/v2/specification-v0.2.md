# AIL Semantic Layer — Спецификация v0.2 (вариант v2)

Статус: предложение (ветка аудита). Нормативная реализация: `mvp/src/v2/`. Отличия от v0.1 помечены **[v0.2]**.

## 1. Назначение

Единое представление технического кода (Binary AST), его контрактов, отношений между функциями (и в будущем — сущностями контента), эволюции во времени (lineage) и машинного обмена (M2M). **[v0.2]** Все отношения, контракты и метрики являются *функциями кода и геометрии*, а не входными данными.

## 2. Слои

```text
L0  Alphabet + Geometry   (alphabet, germes)      — чистые данные
L1  Binary AST            (ast)                   — исполняемый граф операций
L2  Semantics             (symbolic, ownership)   — эффекты, контракты, обязательства
L3  Relations             (derive, relation)      — выведенный граф, насыщенность, степени
L4  Evolution             (contract, lineage)     — refinement, Merkle-цепь
L5  Execution             (vm, actor)             — исполнение, hot-swap, здоровье
L6  Exchange / View       (M2M, VizScene)         — обмен и визуализация
```

## 3. Алфавит Star Gate (нормативная таблица)

| Символ | Имя | Unicode | Клавиша | Приор | Напр. | Допустимые kinds |
|--------|-----|---------|---------|-------|-------|------------------|
| Dot | dot | `·` | `.` | 0.00 | нет | Control, Data |
| Simple | simple | `—` | `"` | 0.00 | нет | Control, Data |
| Directed | directed | `→` | `>` | 0.10 | да | Control, Data |
| Angle | angle | `∠` | `(` | 0.30 | да | Control, Data |
| Triangle | triangle | `△` | `*` | 0.35 | да | Data |
| Diamond | diamond | `◇` | `'` | 0.40 | нет | Data, Ownership |
| Shared | shared | `□̸` | `&` | 0.40 | нет | Ownership |
| Unique | unique | `▲` | `!` | 0.50 | да | Ownership |
| Conflict | conflict | `✕` | `)` | 0.85 | нет | Ownership |
| Hyper | hyper | `⬡` | `#` | 0.90 | нет | HyperMember |

Модификаторы (не рёбра): `%` — принудительно D3; `$` — команда «рычаг». Приор — декларированная насыщенность в `[0,1]`; `prior_degree` при порогах 0.25/0.75 совпадает с «рекомендованной степенью» v0.1.

## 4. Насыщенность и степень (нормативные формулы)

Пусть `w(x,y)` — суммарный вес рёбер между `x` и `y` (0 если рёбер нет), `N(x)` — соседи `x` без узлов вида `Module` (если `exclude_modules_from_topology`).

```text
U(u,v)        = (N(u)∖{v}) ∪ (N(v)∖{u})
C_w(u,v)      = Σ_{x∈U} min(w(u,x), w(v,x))
J_w(u,v)      = C_w / Σ_{x∈U} max(w(u,x), w(v,x))         (0 при пустом U)
topology(u,v) = J_w · (1 − exp(−C_w / support_scale))        support_scale = 2
usage(e)      = 1 − exp(−max(w_e − 1, 0) / usage_scale)      usage_scale = 3
prior(e)      = приор символа (0, если use_prior = false)
saturation(e) = 1 − (1 − topology)(1 − usage)(1 − prior)
degree(e)     = D1 : s < d2_low (0.25) | D2 : s < d3_low (0.75) | D3 : иначе
forced_dense  ⇒ degree = D3
```

Инварианты: `0 ≤ s ≤ 1`; `s` не убывает при росте любой компоненты; `reinforce(e, δ ≥ 0)` не уменьшает `s(e)`.

## 5. Binary AST **[v0.2]**

```rust
enum ParamType { I64, Bool, Object(TypeTag) }
enum Op {
  ConstI64, ConstBool, Move, Consume, AddChecked, SubChecked, Ge,
  LoadField, StoreField, Assert,
  Call { func: String, args: Vec<Reg>, dest: Option<Reg> },   // новое
  Return { value: Option<Reg> },                             // None ⇒ Unit
}
struct Function { name, params: Vec<ParamType>, regs: u16, body: Vec<Op> }
struct Module   { name, functions: Vec<Function> }
```

Ограничение v0.2: тело прямолинейно (нет ветвлений/циклов).

## 6. Эффекты и контракты **[v0.2]**

Символьные значения `Sym ::= Param(i) | Const | ConstBool | Field(Sym, f) | Add | Sub | Ge | Call(name, args) | Unknown`.

`Effects(f)`: `reads ⊆ Loc`, `writes : Loc → Sym` (пост-состояние), `guards ⊆ Sym`, `calls`, `ret`, `typed_reads/typed_writes ⊆ (TypeTag, field)`.

Нормализация `effects_in(f, m)`: вызовы *чистых* функций (`writes = ∅`) заменяются их возвращаемым выражением с подстановкой аргументов (глубина ≤ 8).

**Refinement** `new ⊑ old` (per function `f ∈ old`):

- нарушения: `f ∉ new`; арность/типы параметров ≠; `guards(old) ⊄ guards(new)`; `dom writes(new) ⊄ dom writes(old)`;
- семантические изменения: `guards(new) ∖ guards(old)`; `writes(new)[l] ≠ writes(old)[l]`; `ret ≠`; `reads(new) ∖ reads(old)`;
- структурные изменения: `calls ≠`; добавленные функции.

`new` допустим к hot-swap ⇔ нарушений нет.

## 7. Ownership **[v0.2]**

Состояния регистра: `Uninit | Owned | Consumed`. Ошибки: `UseBeforeInit, UseAfterConsume, DoubleConsume, InvalidReg, ReturnConsumed, MissingReturn, UnreachableAfterReturn`. Предупреждения: мёртвые записи.

Обязательства: `DistinctMutableParams{a,b}` для каждой пары изменяемых объектных параметров одного типа. Проверяется VM при вызове: `args[a] ≠ args[b]`, иначе `AliasedUniqueParams`.

Соответствие v0.1: `BorrowedMut` ≈ изменяемый объектный параметр; `Borrowed` ≈ только читаемый; `✕` ≈ нарушение обязательства.

## 8. Выведенный граф **[v0.2]**

Узлы: функции (`Function`), модуль (`Module`). Рёбра по правилам §5 документа `04-v2-proposal.md`: `✕` (пишут общее поле), `▲` (writer → reader), `□̸` (читают общее), `→` (вызов), `⬡` (членство). Одно ребро на `(пара, kind)`; вес = число общих локаций.

## 9. Здоровье

`transitivity = 3·T / W` (T — треугольники, W — «wedges» среди не-Module узлов); `Lever(C, I, d2_share, C·I, tilted)` по долям D3/D1 среди не-⬡ рёбер; `Zone ∈ {Sparse, Critical, Rigid}` — по доминирующей доле.

## 10. Lineage **[v0.2]**

```text
content_hash(m) = SHA256("ail/v2/module\0" ‖ json(m))
id(g)           = SHA256("ail/v2/generation\0" ‖ content_hash ‖ "\0" ‖ parent_id ‖ "\0" ‖ be32(generation))
```

`verify_links` проверяет последовательность поколений, ссылки и id; `verify(modules)` — ещё и хэши содержимого.

## 11. Hot-swap **[v0.2]**

См. `04-v2-proposal.md` §6. Политика: `reject_on_violation`, `reject_on_ownership_error`, `reject_on_critical`, `saturation: SaturationPolicy`. Результат — `SwapReport` (значение), решение — `Accept | AcceptWithWarnings | Reject`.

## 12. M2M **[v0.2]**

```ebnf
line     := empty | comment | relation | hyper | command
relation := IDENT GLYPH IDENT { "%" }
hyper    := IDENT "#" IDENT { IDENT }
command  := IDENT { ARG }
GLYPH    := '"' | '.' | '>' | '(' | '*' | "'" | '&' | '!' | ')' | UNICODE_SYMBOL
```

`Lnk a g b` ≡ `a g b`. Сериализация использует клавиши; round-trip обязателен.

## 13. GERMES

Вершины Q³ = {−1,+1}³; проекция на плоскость ⟂ (1,1,1) в базисе `e₁=(1,−1,0)/√2, e₂=(1,1,−2)/√6`. Полюса `---`, `+++` → центр. Остальные шесть — правильный шестиугольник, порядок против часовой: `+--, ++-, -+-, -++, --+, +-+`. Антипод — отрицание координат; на шестиугольнике — 180°. Треугольники: веса 1 и 2.

## 14. Глоссарий

| Термин | Определение |
|--------|-------------|
| Насыщенность (saturation) | число в `[0,1]` по §4; свойство ребра |
| Степень связности (degree) | D1/D2/D3 — классификация насыщенности по порогам |
| Топология | компонента насыщенности из общих соседей |
| Использование (usage) | компонента из веса/феромона ребра |
| Приор | декларированная компонента из символа |
| Вес (weight) | неотрицательное число на ребре; 1 — нейтрально |
| Контракт | `Effects` функции + типы параметров |
| Refinement | отношение «новый модуль не ослабляет старый» |
| Обязательство | условие вызова, не доказуемое статически |
| Поколение | запись lineage с `id`, `content_hash`, `parent_id` |
| Transitivity | `3T/W`, глобальный коэффициент кластеризации |
| Lever | баланс долей D3 и D1 |
