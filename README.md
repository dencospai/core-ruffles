# CORE-RUFFLE

Core experimental para executar jogos flash (.swf) no RetroArch usando o Ruffle (https://github.com/ruffle-rs/ruffle).

Versões disponíveis:

- **Windows x64**: `ruffle_libretro.dll`.
- **Linux x64**: `ruffle_libretro.so` — **não testado no RetroArch**.

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