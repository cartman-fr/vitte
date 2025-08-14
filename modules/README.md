# Modules — la forge

Tout est conçu **petit mais costaud**. Tu peux importer module par module, ou tout prendre via la façade :

```vitte
use exports::prelude::*;
```

## Sommaire express

- **log** : logger structuré (texte/JSON), multi-sinks, rate-limit  
- **config** : config à couches (env → TOML/JSON → overrides)  
- **metrics** : counters/gauges/histograms, export **Prometheus**  
- **taskpool** : pool de threads, `spawn()`/`scope()`/`shutdown()`  
- **cache** : LRU + TTL, `get_or_load`, janitor  
- **kvstore** : KV append-only (JSONL), index RAM + compaction  
- **http_client** : HTTP/1.1 minimal (GET/POST/JSON), timeouts, redirects  
- **eventbus** : pub/sub en mémoire (topics `str`)  
- **feature_flags** : flags bool + rollouts (%)  
- **validate** : email/url/uuid, ranges, non-empty  
- **cli** : sous-commandes propres (`App`, `Cmd`, `Ctx`)  
- **channel** : MPMC queue (condvar)  
- **scheduler** : `every(ms)` / `daily HH:MM`  
- **retry** : backoff exponentiel + jitter  
- **rate_limiter** : token-bucket  
- **idgen** : Snowflake 64-bit  
- **uuid** : UUID v4  
- **random** : RNG helpers (ints, floats, choice, shuffle)  
- **csv / ini / yaml_lite** : parseurs légers  
- **checksum** : Adler32, Murmur3-32  
- **rle** : compression RLE  
- **pool** : object pool  
- **prioq** : file de priorité (BinaryHeap)  
- **fs_atomic** : écriture atomique + lockfile  
- **supervisor** : relance de process crashé  
- **plugin** : registre de plugins  
- **migrate** : système de migrations séquencées  
- **tracing** : spans/events (export JSONL)  
- **pagination** : pages + token simple  
- **result_ext** : `Result`/`Option` helpers  
- **stringx** : slug, levenshtein, truncate  
- **mathx** : stats (mean/median/stddev), `clamp`, `lerp`  
- **graph** : DAG + topological sort

## Démarrage rapide

```vitte
use exports::prelude::*;

fn main(_args:[str]) -> int {
    // Logs
    exports::init_default_logging("myapp", log::Level::Info);
    info("bonjour, monde");

    // Config (env + fichier optionnel)
    let cfg = Config::from_env("APP_")
        .merge(Config::from_file("config.toml").unwrap_or(Config::empty()));
    let port = cfg.get_u32("server.port", 8080);

    // Métriques
    let reg = exports::metrics();
    let hits = reg.counter("http_requests_total", "Total des requêtes");

    // Rate-limit 5 QPS
    let rl = TokenBucket::new(5, 10);

    // HTTP simple
    let http = HttpClient::new().timeout_ms(3000);
    if rl.allow() {
        if let Ok(rsp) = http.get("https://example.org") {
            info(format!("status={}", rsp.status));
            hits.inc();
        }
    }

    // Cache TTL
    let mut c = TtlLru::<str,str>::with_capacity(1024);
    let v = c.get_or_load("motd".into(), 60_000, || "Salut Vincent 👋".into());
    debug(format!("motd={}", v));

    0
}
```

## Conseils d’assemblage

- **Imports** : dans les binaires, fais `use exports::prelude::*` pour coder sans te répéter.  
- **CI** : `clippy.toml`, `.rustfmt.toml`, `deny.toml` et `rust-toolchain.toml` sont déjà posés pour garder le chantier net.  
- **Sélection** : si un module ne te sert pas, retire-le de `exports.vitte` (façade lean = liens plus rapides).  
- **Évolution** : ajoute ici tes briques maison, puis expose-les proprement dans la section *Prelude*.

_Classique dans la forme, nerveux dans le fond. Tu écris, ça compile, ça déploie. Tout droit._
