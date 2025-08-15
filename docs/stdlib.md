# Vitte Standard Library — Référence Ultra Complète

> _“Des briques sobres, des garanties nettes, des perfs honnêtes.”_  
Dernière mise à jour : **2025-08-15** • Édition du langage : **2025** • Toolchain minimal : **≥ 0.6.0**

---

## Sommaire

- [Philosophie & garanties](#philosophie--garanties)  
- [Import & prélude](#import--prélude)  
- [Stabilité & availability](#stabilité--availability)  
- [Compatibilité plateformes & backends](#compatibilité-plateformes--backends)  
- [Concurrence & mémoire](#concurrence--mémoire)  
- [Erreurs, panics & résultats](#erreurs-panics--résultats)  
- [Catalogue des modules](#catalogue-des-modules)
  - [Noyau (core)](#noyau-core)
  - [Système & I/O](#système--io)
  - [Réseau](#réseau)
  - [Concurrence](#concurrence)
  - [Formats & parsing légers](#formats--parsing-légers)
  - [Utilitaires numériques & chaînes](#utilitaires-numériques--chaînes)
  - [Collections, structures & graphes](#collections-structures--graphes)
  - [Observabilité & diagnostique](#observabilité--diagnostique)
  - [Orchestration & jobs](#orchestration--jobs)
  - [Persistance & migration](#persistance--migration)
  - [Extensions & plugins](#extensions--plugins)
- [Sous-ensemble embarqué / no_std](#sous-ensemble-embarqué--no_std)  
- [Aspects sécurité](#aspects-sécurité)  
- [Conventions de nommage](#conventions-de-nommage)  
- [Évolution & RFC](#évolution--rfc)
- [Annexes — API détaillées (extraits)](#annexes--api-détaillées-extraits)
- [Exemples rapides](#exemples-rapides)

---

## Philosophie & garanties

- **Sécurité d’abord** : pas de _null_ implicite ; API publiques => `Option[T]` / `Result[T,E]`.
- **Zéro coût caché** : allocations, I/O, verrous, `spawn` sont explicites dans les signatures.
- **Sync/Async lisible** : variantes asynchrones suffixées `*_async` (MVP).
- **Stabilité graduelle** : `stable` / `preview` / `experimental` par module et par fonction.
- **Thread-safety documentée** : types conceptuels `Send` / `Sync` précisés.

---

## Import & prélude

Le **prélu** expose de base : types numériques, `bool`, `print`, `assert`, conversions usuelles.

Importer un module :
```vitte
use std::fs
use std::net
use modules::kvstore     // modules additionnels fournis dans le repo
```

> Selon le packaging, `modules/*` peut être regroupé dans la stdlib, ou distribué via `vitpm`.

---

## Stabilité & availability

- **stable** : API gelée, compat garantie (patch/minor).
- **preview** : API plausible, retours attendus, peut bouger.
- **experimental** : aucune garantie, peut casser.

Disponibilité (icônes) : 🖥 desktop • 🛠 server • 🔌 embedded • 🧪 kernel • 🌐 wasm

---

## Compatibilité plateformes & backends

| Backend / Plateforme | VM (bytecode) | LLVM (native) | Cranelift (JIT) |
|---:|:---:|:---:|:---:|
| Linux x86_64 | ✅ | ✅ | ✅ |
| macOS (Intel/Apple) | ✅ | ✅ | ✅ |
| Windows x64 | ✅ | ✅ | ✅ |
| BSD | ✅ | ✅ | ✅ |
| WASM | ⚠️ partiel | n/a | n/a |
| Embedded (ARM/RISC-V) | ✅ | ✅ | n/a |

> ⚠️ WASM : `fs`, `process` indisponibles ; `net` restreint (fetch-like).

---

## Concurrence & mémoire

- **RAII + emprunts** ; pas de GC global.
- `thread::spawn` → thread OS ; `*_async` → runtime coopératif (MVP).
- Verrous : `Mutex`, `RwLock`, `Once`.  
- Canaux : `channel` MPSC/MPMC ; `try_*` = non bloquant.

---

## Erreurs, panics & résultats

- Public API : **pas** de `panic!` pour erreurs prévisibles → utiliser `Result`.  
- `panic!` réservé aux invariants internes violés.  
- Pattern recommandé :
```vitte
do run() -> Result[Unit, Error] {
  let cfg = config::load("app.ini")?
  http_client::get(cfg.endpoint)?
  Ok(())
}
```

---

## Catalogue des modules

> Pour chaque module : **But**, **API clé**, **Garanties**, **Complexité** (si pertinent), **Erreurs**, **Exemple**.

### Noyau (core)

#### `prelude` (stable, 🖥🛠🔌🧪🌐)
- **But** : ergonomie (`print`, `eprintln`, `assert`, conversions).
- **API** :
  ```vitte
  do print(x: any)
  do eprintln(x: any)
  do assert(cond: bool, msg: str?)
  ```
- **Exemple** :
  ```vitte
  print("Hello"); assert(1+1==2, "math broke")
  ```

#### `collections` (stable, 🖥🛠🔌🧪🌐)
- `Vec[T]`, `Map[K,V]`, `Set[T]`
- **Complexité** : `Vec.push` amorti O(1) ; hash-map O(1) attendu.

#### `string` (stable, 🖥🛠🔌🧪🌐)
- `String` propriétaire / `str` vue immuable ; `split`, `join`, `replace`, etc.

#### `result`, `option` (stable, 🖥🛠🔌🧪🌐)
- Helpers `map`, `map_err`, `unwrap_or` ; propagation `?` (proposée).

---

### Système & I/O

#### `fs` (stable, 🖥🛠🧪)
- **API** :
  ```vitte
  do read_to_string(path: str) -> Result[String, FsError]
  do write_atomic(path: str, data: []u8) -> Result[Unit, FsError]
  do remove_file(path: str) -> Result[Unit, FsError]
  do exists(path: str) -> bool
  do create_dir_all(path: str) -> Result[Unit, FsError]
  ```
- **Garanties** : `write_atomic` = temp + flush + rename (best-effort cross-platform).

#### `fs_atomic` (preview, 🖥🛠)
- Écritures crash-safe ; note antivirus Windows (fallback documenté).

#### `io` (stable, 🖥🛠🔌🧪)
- Buffers, readers/writers, stdin/stdout/stderr.

#### `process` (preview, 🖥🛠)
- Lancement de commandes, pipes, env. **Non dispo** en WASM/embedded.

#### `time` (stable, 🖥🛠🔌🧪🌐)
- `Instant`, `Duration`, horloges monotone/temps-réel.

---

### Réseau

#### `net` (preview, 🖥🛠🌐)
- TCP/UDP de base (sync + async selon runtime).

#### `http_client` (preview, 🖥🛠🌐)
- **API** :
  ```vitte
  do get(url: str) -> Result[Response, HttpError]
  do post(url: str, body: []u8, headers: Map[str,str]?) -> Result[Response, HttpError]
  struct Response { status: u16, headers: Map[str,str], body: []u8 }
  ```
- **Features** : `tls` (optionnel), `gzip` (optionnel).

---

### Concurrence

#### `thread` (stable, 🖥🛠)
- `spawn`, `join`, `sleep`.

#### `channel` (stable, 🖥🛠)
- **API** :
  ```vitte
  do channel[T](capacity: usize?) -> (Sender[T], Receiver[T])
  do send(self &Sender[T], v: T) -> Result[Unit, ChannelError]
  do try_send(self &Sender[T], v: T) -> Result[Unit, ChannelError]
  do recv(self &Receiver[T]) -> Result[T, ChannelClosed]
  do try_recv(self &Receiver[T]) -> Result[T, ChannelError]
  ```
- **Garanties** : MPMC, FIFO par producteur.

#### `taskpool` (preview, 🖥🛠)
- Pool de workers, planification simple (`spawn` sur N threads).

#### `scheduler` (preview, 🖥🛠)
- Tâches différées / périodiques (cron-like).

#### `retry` (preview, 🖥🛠🌐)
- Backoff exponentiel + jitter ; **idempotence** côté appelant.

#### `rate_limiter` (preview, 🖥🛠🌐)
- Token-bucket / leaky-bucket thread-safe.

---

### Formats & parsing légers

#### `csv` (stable, 🖥🛠🌐)
- Parsing streaming ; séparateurs `,` / `;`.

#### `ini` (stable, 🖥🛠🔌)
- Sections, clés/valeurs, `;`/`#` commentaires.

#### `yaml_lite` (preview, 🖥🛠)
- Sous-ensemble : scalaires, maps, listes.

#### `checksum` (stable, 🖥🛠🔌)
- `crc32`, `adler32` ; (SHA-like en preview).

#### `rle` (stable, 🖥🛠🔌)
- Run-Length Encoding minimal.

---

### Utilitaires numériques & chaînes

#### `random` (stable, 🖥🛠🔌🧪🌐)
- PRNG XorShift/PCG documenté ; `uniform`, `rand_range`.

#### `uuid` (stable, 🖥🛠🌐)
- v4 (aléatoire), **v7** (time-ordered) en preview.

#### `idgen` (preview, 🖥🛠🌐)
- Snowflake-like (timestamp + machine + seq).

#### `stringx` (stable, 🖥🛠🌐)
- `trim`, `pad`, `slugify`, `case_convert`.

#### `mathx` (stable, 🖥🛠🔌🧪🌐)
- `clamp`, `lerp`, `gcd`, `lcm`, stats simples.

---

### Collections, structures & graphes

#### `pool` (preview, 🖥🛠)
- Pools d’objets réutilisables (moins d’allocs).

#### `prioq` (stable, 🖥🛠🔌)
- File à priorité (binary heap).

#### `graph` (preview, 🖥🛠)
- Adjacences, BFS/DFS, Dijkstra (MVP).

---

### Observabilité & diagnostique

#### `log` (stable, 🖥🛠🔌🧪🌐)
- Niveaux `trace`→`error`, handlers multiples, env : `VITTE_LOG=info`.

#### `metrics` (preview, 🖥🛠🌐)
- Compteurs, jauges, histos ; export Prometheus-like.

#### `tracing` (preview, 🖥🛠🌐)
- Spans, propagation de contexte.

#### `pagination` (stable, 🖥🛠🌐)
- Pagination offset/limit ou cursor.

#### `result_ext` (stable, 🖥🛠🔌🧪🌐)
- Helpers : `tap_ok`, `tap_err`, `or_else`, etc.

---

### Orchestration & jobs

#### `supervisor` (preview, 🖥🛠)
- Redémarrages supervisés, arbres à la Erlang.

#### `eventbus` (preview, 🖥🛠🌐)
- Pub/Sub en mémoire, sujets, wildcards.

#### `feature_flags` (preview, 🖥🛠🌐)
- Flags dynamiques (sources : env, fichier, HTTP).

#### `cli` (stable, 🖥🛠🔌🧪🌐)
- Parsing d’arguments, sous-commandes, auto-help.

#### `validate` (stable, 🖥🛠🌐)
- Schémas simples (types, min/max, regex) pour valider des structures.

---

### Persistance & migration

#### `kvstore` (preview, 🖥🛠)
- KV embarqué (mémoire + disque simple), TTL.

#### `cache` (preview, 🖥🛠)
- LRU/LFU ; MVP : LRU.

#### `migrate` (preview, 🖥🛠)
- Migrations séquentielles `Vxxx__desc`, hooks up/down.

---

### Extensions & plugins

#### `plugin` (experimental, 🖥🛠)
- Chargement dynamique ; sandbox recommandé.  
- ⚠️ Surface d’attaque/ABI ; désactivé par défaut en WASM/embedded.

---

## Sous-ensemble embarqué / no_std

Disponibles : `prelude`, `string`, `collections` (réduit), `random`, `checksum`, `rle`, `mathx`, `cli` (partiel), `time` (monotone).  
Indispo : `process`, `fs` (selon cible), `http_client` (DOM-like seulement en WASM).

Exemple blink (pseudo) :
```vitte
use embedded::gpio
do main(){
  let led = gpio::pin(13)
  loop { led.toggle(); time::sleep(200.ms) }
}
```

---

## Aspects sécurité

- **Entrées non fiables** : valider (ex : `validate`).
- **FFI** : wrappers sûrs ; propriété des buffers **documentée**.
- **Fichiers** : `fs_atomic` pour écriture critique.
- **Réseau** : activer `tls` en production.

---

## Conventions de nommage

- Sync : `read_to_string` • Async : `read_to_string_async`  
- Non-bloquant : préfixe `try_` (`try_recv`)  
- Invariants forts : suffixe `*_strict` si pertinent.

---

## Évolution & RFC

- Toute nouvelle API passe par `rfcs/` (`0000-template.md`).  
- Promotions : `experimental → preview → stable` avec période de gel.  
- Ruptures regroupées par **édition** du langage.

---

## Annexes — API détaillées (extraits)

### `fs`
```vitte
type Path = str
enum FsError { NotFound, Permission, Exists, Io(str) }

do read(path: Path) -> Result[[]u8, FsError]
do read_to_string(path: Path) -> Result[String, FsError]
do write(path: Path, data: []u8) -> Result[Unit, FsError]
do exists(path: Path) -> bool
do create_dir_all(path: Path) -> Result[Unit, FsError]
do remove_file(path: Path) -> Result[Unit, FsError]
```

### `http_client`
```vitte
enum HttpError { Dns, Connect, Tls, Timeout, Status(u16), Proto(str) }
struct Response { status: u16, headers: Map[str,str], body: []u8 }

do get(url: str) -> Result[Response, HttpError]
do post(url: str, body: []u8, headers: Map[str,str]?) -> Result[Response, HttpError]
do get_async(url: str, timeout_ms: u32?) -> Result[Response, HttpError]
```

### `channel`
```vitte
enum ChannelError { Full, Closed }
enum ChannelClosed { Closed }

do channel[T](capacity: usize?) -> (Sender[T], Receiver[T])
do send(self &Sender[T], v: T) -> Result[Unit, ChannelError]
do try_send(self &Sender[T], v: T) -> Result[Unit, ChannelError]
do recv(self &Receiver[T]) -> Result[T, ChannelClosed]
do try_recv(self &Receiver[T]) -> Result[T, ChannelError]
```

### `cache`
```vitte
struct LruCache[K,V]
do with_capacity[K,V](n: usize) -> LruCache[K,V]
do get(self &mut, k: &K) -> Option[&V]
do put(self &mut, k: K, v: V) -> Option[V]
do remove(self &mut, k: &K) -> Option[V]
```

### `uuid`
```vitte
type Uuid = [16]u8
do v4() -> Uuid
do v7(now_ms: u64, rand: []u8[10]) -> Uuid // preview
```

### `validate`
```vitte
struct Schema
do string(min: usize?, max: usize?, pattern: str?) -> Schema
do integer(min: i64?, max: i64?) -> Schema
do object(fields: Map[str, Schema], required: []str?) -> Schema
do validate(schema: Schema, value: any) -> Result[Unit, str]
```

### `metrics`
```vitte
struct Counter; struct Gauge; struct Histo
do counter(name: str, labels: Map[str,str]?) -> Counter
do inc(self &Counter, by: u64?)
do gauge(name: str, labels: Map[str,str]?) -> Gauge
do set(self &Gauge, v: f64)
do histo(name: str, buckets: []f64) -> Histo
do observe(self &Histo, v: f64)
```

### `feature_flags`
```vitte
enum Source { Env, File(str), Http(str) }
do is_enabled(key: str, default: bool?) -> bool
do reload(source: Source) -> Result[Unit, str]
```

---

## Exemples rapides

```vitte
// Logging + HTTP + Retry
use log; use retry; use http_client

do fetch_with_retry(url: str) -> Result[String, str] {
  let policy = retry::exponential_backoff(max_retries: 5, base_ms: 100, jitter: true)
  let res = retry::run(policy, || http_client::get(url))
  match res {
    Ok(r)  => Ok(string::from_bytes(r.body)),
    Err(e) => Err(log::error(e))
  }
}
```

```vitte
// Channels + Taskpool
use channel; use taskpool

do main() {
  let (tx, rx) = channel::channel
  let pool = taskpool::with_threads(4)
  for i in 0..1000 { pool.spawn({ tx.send(i) }) }
  let mut sum = 0
  loop {
    match rx.recv() { Ok(v) => sum += v, Err(_) => break }
  }
  print(sum)
}
```

```vitte
// Validation + Config + CLI
use validate; use config; use cli

do main() -> i32 {
  let args = cli::parse()      // --port 8080 --host 0.0.0.0
  let schema = validate::object({
     "host": validate::string(pattern: r"^\\d+\\.\\d+\\.\\d+\\.\\d+$"),
     "port": validate::integer(min: 1, max: 65535)
  }, required: ["host","port"])

  let cfg = config::from_args(args)?
  validate::validate(schema, cfg)?
  print("Config OK")
  0
}
```
