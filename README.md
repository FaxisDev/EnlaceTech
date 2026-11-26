# EnlaceTech

App de escritorio (macOS, Windows y Linux) para guardar enlaces de Rapidgator y
descargarlos en cuanto se pueda.

- Guarda los enlaces que pegues.
- Cada cierto tiempo pregunta a Rapidgator si ya se pueden descargar.
- Si se puede, muestra el botón **Descargar**. Si no, muestra el motivo: una
  explicación en español y el mensaje original de Rapidgator.
- Descarga a la carpeta que elijas (por defecto, Descargas del sistema).
- Envía una notificación del sistema cuando un enlace que estaba en espera ya
  se puede descargar (se puede apagar en Ajustes).

Hecha con Tauri 2, React 19 y Material UI 9.

## Requisitos

- Node.js 20 o superior.
- Rust (https://rustup.rs).
- Los requisitos de Tauri para tu sistema: https://tauri.app/start/prerequisites/
- En Linux, además, `libdbus-1-dev` (lo usa el llavero del sistema).

Hasta ahora solo se ha compilado y probado en macOS.

## Uso

```bash
npm install
npm run tauri dev      # abre la app en modo desarrollo
npm run tauri build    # genera el instalador para el sistema en el que estás
```

La primera compilación tarda unos minutos; las siguientes, segundos.

## Instalador

`npm run tauri build` deja los resultados en `src-tauri/target/release/bundle/`.
En macOS:

- `dmg/EnlaceTech_<versión>_<arquitectura>.dmg`: el instalador. Se abre y se
  arrastra EnlaceTech a Aplicaciones.
- `macos/EnlaceTech.app`: la app sin empaquetar.

El instalador solo sirve para la arquitectura del equipo donde se compiló
(`aarch64` es Apple Silicon). Para Macs con Intel:
`npm run tauri build -- --target x86_64-apple-darwin`.

La app lleva una firma local (ad hoc), suficiente para usarla en el Mac donde
se compiló. Para repartirla a otras personas hace falta firmarla con una cuenta
de Apple Developer y notarizarla; sin eso, macOS la bloquea al descargarla.

`CI=true npm run tauri build` genera el `.dmg` sin pedir permiso para controlar
Finder (el instalador queda igual, sin la disposición decorada de la ventana).

`npm run dev` abre solo la interfaz en el navegador, con datos de ejemplo, para
trabajar en el diseño sin compilar Rust. Añade `?signedout` o `?empty` a la URL
para ver esos estados.

```bash
cd src-tauri && cargo test   # pruebas del backend
```

## Cómo funciona

Usa la API oficial de Rapidgator (https://rapidgator.net/article/api):

| Para qué | Llamada |
| --- | --- |
| Iniciar sesión y obtener el token | `user/login` |
| Datos de la cuenta (plan, tráfico) | `user/info` |
| Saber si los enlaces existen (hasta 25 por llamada) | `file/check_link` |
| Pedir el enlace de descarga, o el motivo del rechazo | `file/download` |

La API no tiene una página donde copiar un token: el token se obtiene al
iniciar sesión con correo y contraseña, y caduca. Por eso la app pide iniciar
sesión en **Ajustes**.

Un enlace se considera descargable cuando `file/download` devuelve una URL. Si
devuelve un error, ese mensaje es el motivo que se muestra.

Para no cargar los servidores de Rapidgator, como pide su documentación:

- El intervalo mínimo entre comprobaciones automáticas es de 5 minutos.
- Los enlaces que ya están disponibles no se vuelven a pedir en cada ronda;
  solo se confirma que siguen existiendo.
- Si la sesión caduca, se renueva una sola vez. Si Rapidgator rechaza la
  contraseña, no se insiste (los intentos repetidos bloquean la IP).

## Avisos

Cuando una comprobación encuentra enlaces que pasaron a estar disponibles, la
app envía una notificación con sus nombres. No la envía si la ventana de EnlaceTech
está en primer plano, porque ahí ya se ve en la lista.

En macOS, con `npm run tauri dev` las notificaciones llegan a nombre de
**Terminal**, porque la app aún no está empaquetada. Con la app instalada
(`npm run tauri build`) llegan a nombre de EnlaceTech.

## En segundo plano

Al cerrar la ventana, la app no termina: la ventana se oculta y EnlaceTech
sigue comprobando desde su icono en la barra de menús (macOS) o en la bandeja
del sistema (Windows y Linux).

- El menú del icono muestra cómo van los enlaces y permite **Abrir
  EnlaceTech**, **Comprobar ahora** y **Salir**.
- Para cerrar la app del todo: **Salir** en ese menú, o Cmd+Q en macOS.
- En macOS, un clic en el icono del Dock también vuelve a mostrar la ventana.
- Se puede desactivar en **Ajustes → En segundo plano**; entonces cerrar la
  ventana cierra la app.
- Solo hay una copia de la app a la vez: abrirla de nuevo muestra la ventana
  de la que ya está en marcha.
- Si el sistema no tiene bandeja (algunos escritorios de Linux), cerrar la
  ventana cierra la app, para no dejarla oculta sin forma de recuperarla.

## Dónde se guardan las cosas

- **Sesión y contraseña:** en el llavero del sistema (Keychain en macOS,
  Credential Manager en Windows, Secret Service en Linux). Nunca en archivos.
  Guardar la contraseña es opcional.
- **Enlaces y ajustes:** en `enlacetech.json`, dentro de la carpeta de datos de la
  app (`~/Library/Application Support/com.enlacetech.desktop/` en macOS).

## Estructura

```
src/                    interfaz (React + Material UI)
  theme.ts              tema azul marino
  components/           barra lateral, lista de enlaces, ajustes
  lib/bridge.ts         llamadas al backend
  lib/mock.ts           datos de ejemplo, solo en desarrollo
src-tauri/src/          backend (Rust)
  rapidgator.rs         cliente de la API
  engine.rs             lista de enlaces, comprobaciones y descargas
  secrets.rs            llavero del sistema
  tray.rs               icono y menú de la barra de menús / bandeja
  lib.rs                comandos que la interfaz puede llamar
```

## Licencia

[MIT](LICENSE).
