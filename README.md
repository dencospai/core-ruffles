<<<<<<< HEAD
# Core-Ruffle
Core experimental do Ruffle Flash Emulator para executar jogos Flash (.swf) no RetroArch
=======
# CORE-RUFFLE

Core experimental para executar jogos Flash (`.swf`) no RetroArch usando o
[Ruffle](https://github.com/ruffle-rs/ruffle).

Versões disponíveis:

- **Windows x64**: `ruffle_libretro.dll`.
- **Linux x64**: `ruffle_libretro.so` — **não testado no RetroArch**.

## Gerar o core para Windows x64

Instale Git, Rust pelo [rustup](https://rustup.rs/), Java 17 e o **Visual Studio
2022 Build Tools** com a carga **Desenvolvimento para desktop com C++**, MSVC x64
e Windows SDK. Deixe `cargo`, `rustup` e `java` disponíveis no `PATH`.
O arquivo `rust-toolchain.toml` seleciona a versão do Rust usada pelo projeto.

Na raiz do projeto, execute pelo PowerShell:

```powershell
.\scripts\build-windows.ps1
```

Os arquivos serão gerados em:

```text
dist/windows-x64/ruffle_libretro.dll
dist/windows-x64/ruffle_libretro.info
```

## Gerar o core para Linux x64

Em um Linux x64 com glibc, instale Git, Rust pelo [rustup](https://rustup.rs/),
Java 17 e um compilador/linker C/C++. Deixe `cargo` e `java` disponíveis no `PATH`.

Na raiz do projeto, execute:

```sh
bash scripts/build-linux.sh
```

Os arquivos serão gerados em:

```text
dist/linux-x64/ruffle_libretro.so
dist/linux-x64/ruffle_libretro.info
```

A primeira compilação requer acesso à internet para baixar as dependências.
Rust, Java e as ferramentas de compilação não são necessários para usar o core pronto.

## Instalar no RetroArch — Windows x64

1. No RetroArch, consulte **Configurações → Diretório** para localizar as pastas
   de **cores** e de **informações dos cores**. Depois feche o RetroArch.
2. Copie `ruffle_libretro.dll` para a pasta de cores.
3. Copie `ruffle_libretro.info` para a pasta de informações dos cores.
4. Abra o RetroArch x64 e selecione **Carregar core → Ruffle**.
5. Selecione **Carregar conteúdo** e abra o jogo `.swf`.

## Instalar no RetroArch — Linux x64

1. No RetroArch, consulte **Configurações → Diretório** para localizar as pastas
   de **cores** e de **informações dos cores**. Depois feche o RetroArch.
2. Copie `ruffle_libretro.so` para a pasta de cores.
3. Copie `ruffle_libretro.info` para a pasta de informações dos cores.
4. Abra o RetroArch x64 e selecione **Carregar core → Ruffle**.
5. Selecione **Carregar conteúdo** e abra o jogo `.swf`.

Os diretórios podem variar entre instalações nativas e Flatpak; use os caminhos
indicados pelo próprio RetroArch.
>>>>>>> principal
