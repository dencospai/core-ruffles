# Publicar o código no GitHub

Publique o código, os scripts, as licenças, a documentação e as duas fixtures
originais de teste. Os jogos utilizados nos testes e as capturas locais não
fazem parte da distribuição do código-fonte.

O `.gitignore` exclui:

- `FLASH GAMES/`, `FLASHGAMES/`, `print/`, coleções de jogos e saves;
- `target/`, `dist/`, `.tools/`, binários, pacotes e logs;
- configurações locais de agentes/editores, pastas de credenciais, arquivos
  `.env`, chaves privadas e backups;
- arquivos SWF, exceto `tests/fixtures/smoke.swf` e `tests/fixtures/cursor.swf`.

As duas exceções são arquivos originais, reproduzíveis por
`scripts/make_test_swf.py` e `scripts/make_cursor_swf.py`.
Mantenha `Cargo.lock`, `LICENSE-MIT`, `LICENSE-APACHE`, `THIRD_PARTY.md` e
`.github/workflows/build.yml` no repositório.

## Antes do primeiro commit

Na pasta que será publicada, use Git para selecionar os arquivos:

```sh
git init
git add .
git status --short
git diff --cached --stat
git diff --cached
```

Confira a lista e o conteúdo antes de fazer commit e push. Não use `git add -f`
para incluir jogos, configurações privadas ou arquivos gerados.

O `.gitignore` só afeta arquivos não rastreados. Ele não remove arquivos de
commits anteriores nem impede adição forçada. Se partir de um repositório
existente, revise também os arquivos rastreados e seu histórico.

## Upload manual

O envio pelo navegador não aplica automaticamente as regras do `.gitignore`.
Para esse caso, use o pacote de código-fonte revisado, quando disponível em
`dist/CORE-RUFFLE-public-source.zip`: extraia e envie somente seu conteúdo,
incluindo os arquivos ocultos `.gitignore` e `.github/`. Não envie a pasta
completa de trabalho nem os jogos/capturas locais.

Esse pacote é uma cópia da revisão local no momento em que foi criado. Refaça
a revisão após alterações, principalmente ao adicionar arquivos ou credenciais.

## Binários em Releases

Se desejar publicar os cores compilados, faça isso separadamente em Releases,
usando os pacotes preparados para distribuição com suas licenças e avisos de
dependências. Não inclua jogos de terceiros, saves, logs ou capturas pessoais.
