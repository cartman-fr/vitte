# Web Echo — Serveur HTTP minimal en Vitte

Ce service expose un mini-HTTP **echo** avec un pipeline de **middlewares**. Il est pensé pour être
ultra-lisible, extensible en 2 minutes, et suffisant pour des demos, tests et intégrations simples.

## 📂 Structure

```
web-echo/
├─ main.vitte         # serveur HTTP + routes
└─ middleware.vitt    # types Request/Response + middlewares (Logger, CORS, …)
```

## 🚀 Démarrage rapide

```bash
# Lancer (port par défaut 8080)
vittec run web-echo/main.vitte

# Port custom
PORT=9090 vittec run web-echo/main.vitte
```

## 🌐 Routes

| Méthode | Chemin     | Description                                   | Réponse |
|--------:|------------|-----------------------------------------------|---------|
| GET     | `/`        | Page d’info minimale                          | 200 text/plain |
| GET     | `/health`  | Liveness probe                                 | 200 "ok" |
| GET     | `/time`    | Heure locale & TZ en JSON                      | 200 application/json |
| ANY     | `/headers` | Retourne les headers & query reçus (JSON)      | 200 application/json |
| POST    | `/echo`    | Ré-émet le corps reçu (Content-Type reflété)   | 200 echo body |

> Le pipeline par défaut inclut : `RequestId` → `Logger` → `Timing` → `Cors(permissive)` → `LimitBody(1MiB)`

## ⚙️ Variables d’environnement

- `PORT` : port d’écoute (défaut `8080`)
- `LOG`  : niveau logs du runtime (`error|warn|info|debug`… selon votre std)

## 🧪 Exemples cURL

```bash
# Santé
curl -i http://localhost:8080/health

# Heure
curl -i http://localhost:8080/time

# Headers
curl -i http://localhost:8080/headers -H 'X-Demo: yes' -H 'Accept: application/json'

# Echo JSON
curl -i http://localhost:8080/echo   -H 'Content-Type: application/json'   -d '{"hello":"vitte"}'

# Echo binaire
head -c 16 /dev/urandom | curl -i http://localhost:8080/echo --data-binary @-   -H 'Content-Type: application/octet-stream'
```

## 🧱 Middlewares fournis

- **Logger** : log `method path ←peer →status (ms)`
- **RequestId** : génère un `x-request-id` (req & res)
- **Timing** : ajoute `Server-Timing: app;dur=…`
- **Cors** : réponses CORS permissives + gestion `OPTIONS`
- **LimitBody(max_bytes)** : 413 si le corps dépasse la limite

### Ajouter un middleware custom

Dans `middleware.vitt` :

```vitte
pub struct PoweredBy;
impl Middleware for PoweredBy {
    fn handle(&self, req: Request, next: Next) -> Response {
        let mut res = next(req);
        res.headers.insert("x-powered-by".into(), "vitte-web-echo".into());
        res
    }
}
```

Puis dans `main.vitte` (construction du pipeline) :

```vitte
let chain = Chain::new()
  .use(RequestId)
  .use(Logger)
  .use(Timing)
  .use(Cors::permissive())
  .use(LimitBody{ max_bytes: 1 * 1024 * 1024 })
  .use(PoweredBy);
```

## ➕ Ajouter une route

Dans `main.vitte`, dans la fonction `router(req)` :

```vitte
("GET", "/hello") => Response::new(200).text("bonjour !"),
```

## 🛡️ Prod tips

- Placez un proxy (Nginx/Traefik/Caddy) devant pour TLS/HTTP2 et limites de débit.
- Ajustez `LimitBody` en fonction de votre use-case.
- Journaux : orientez `stderr` vers journald ou un collector (Loki/Elastic).

## 📦 Docker (exemple)

```Dockerfile
# syntax=docker/dockerfile:1
FROM alpine:3.20
WORKDIR /app
# Copiez votre binaire compilé statiquement (ex: via vittec build --release)
COPY bin/web-echo /app/web-echo
EXPOSE 8080
ENV PORT=8080
ENTRYPOINT ["/app/web-echo"]
```

## 🧰 Systemd (exemple)

```
[Unit]
Description=Vitte Web Echo
After=network-online.target
Wants=network-online.target

[Service]
Environment=PORT=8080
ExecStart=/opt/web-echo
Restart=always
RestartSec=2
NoNewPrivileges=true
ProtectSystem=full
ProtectHome=true

[Install]
WantedBy=multi-user.target
```

## 🐞 Dépannage

- **400 Bad Request** : parser refuse la requête (entête incomplète, encodage cassé).
- **413 Payload Too Large** : augmentez `LimitBody.max_bytes`.
- **CORS** : vérifiez vos entêtes sur `OPTIONS` (middleware `Cors`).

---

Made with ❤️ and a sturdy dose of minimalism.
