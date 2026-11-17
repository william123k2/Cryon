# Cryon 0.2.1

Launcher Vanilla independente para Minecraft Java Edition, com Rust, Tauri 2, React, TypeScript e Vite. Plataforma desta edição: Windows 10/11 x64.

## Instalador Windows x64

Baixe e execute o [instalador Cryon 0.2.1](downloads/Cryon_0.2.1_x64-setup.exe). O pacote foi compilado para Windows x64 e a validação registrada está em [VALIDACAO.md](VALIDACAO.md).

O Windows pode avisar que o editor não foi verificado porque o instalador não tem assinatura digital. O Cryon precisa do Microsoft Edge WebView2. Na primeira instalação de uma versão do jogo, precisa de internet para obter os arquivos oficiais e o runtime Java Temurin; não é necessário instalar Java manualmente.

Confira o SHA-256 em [downloads/SHA256SUMS.txt](downloads/SHA256SUMS.txt).
## Usar
Abra `Cryon.exe` ou instale com o pacote Windows entregue. O computador precisa de Microsoft Edge WebView2; não precisa ter Java instalado.

1. Em **Configurações**, escolha seu nome offline e salve.
2. Em **Instâncias**, configure uma pasta, versão, memória e resolução. Os valores iniciais são 512–2048 MB e 1280 × 720.
3. Em **Versões**, consulte o catálogo oficial, filtre releases/snapshots e escolha uma versão para a instância selecionada.
4. Clique **Instalar** para preparar os arquivos, ou **Jogar** para instalar/verificar e iniciar em uma operação.
5. Acompanhe os arquivos e erros em **Downloads**. O cancelamento preserva arquivos já verificados e descarta os downloads temporários próprios.

O primeiro uso requer internet. Depois de uma versão e seu Java terem sido instalados, o Cryon reutiliza o catálogo e os arquivos locais, verificando seus hashes antes de iniciar. Uma versão desconhecida no catálogo salvo exige atualizar o catálogo com internet.

O perfil é **local/offline**. Não há OAuth Microsoft, autenticação de servidores oficiais ou simulação de login Microsoft. O launcher não concede licença do Minecraft; recursos online e menus disponíveis dependem do próprio cliente. Mantenha o launcher aberto durante o jogo para acompanhar logs e usar o botão **Encerrar**. Para uma saída normal, prefira **Quit Game / Sair do jogo** dentro do Minecraft.

## Funcionalidades implementadas

- Perfil persistente, validação de nome e UUID offline determinístico compatível com Java.
- Configurações e instâncias em JSON versionado, com gravação atômica e backup.
- Criar, editar, selecionar e remover cadastro de instâncias, com confirmação. **A remoção preserva as pastas e os mundos.**
- Seletores nativos de pasta e Java; RAM e resolução por instância.
- Catálogo oficial Mojang, releases, snapshots, busca e indicador de versões instaladas.
- Downloads reais com Reqwest/Tokio, seis transferências simultâneas, bytes, porcentagem, taxa média, até três tentativas, cancelamento e integridade SHA-1/SHA-256.
- Runtime privado Eclipse Adoptium Temurin, escolhido pelo requisito `javaVersion` dos metadados. Se ausente nos metadados antigos, usa Java 8.
- Verificação do Java configurado manualmente ou gerenciado; download e extração segura de ZIP oficial; licenças preservadas no runtime.
- Cliente JAR, bibliotecas, assets, logging oficial, extração de nativas e assets legados quando declarados.
- Regras de SO/arquitetura e recursos, argumentos modernos condicionais e argumentos legados, classpath sem shell e suporte a caminhos com espaços.
- Processo Java real, captura de stdout/stderr, código de saída, logs em disco e exportação de diagnóstico.
- Ao abrir o jogo, minimiza o launcher; ao fechar, restaura o estado original da janela. Encerramentos inesperados exibem um aviso com o código e as mensagens de erro mais recentes.
- Trava exclusiva da pasta de dados para evitar dois launchers gravando a mesma configuração ao mesmo tempo.
- Interface própria azul, cristal original, layout responsivo e estados baseados no backend.

## Dados e recuperação

Os dados ficam em `%LOCALAPPDATA%\CryonData`, separados da pasta do programa:

```text
state.json                 Perfil, preferências, instâncias e seleção
state.backup.json          Cópia anterior à última gravação
cryon.lock                 Trava exclusiva enquanto o launcher usa a pasta
manifest.json              Catálogo oficial salvo
versions/<id>/             Metadados, cliente, nativas e registro de instalação
libraries/                 Bibliotecas compartilhadas e verificadas
assets/                    Índices, objetos, logging e assets virtuais legados
java/<major>/              Runtime privado, licenças e runtime.json
games/                     Padrão das pastas independentes de jogo
logs/                      Log do launcher e logs individuais do Minecraft
```

Se `state.json` estiver inválido, o Cryon informa o erro e **não substitui o arquivo corrompido por configurações vazias**. Feche o aplicativo, preserve uma cópia do arquivo e restaure `state.backup.json` como `state.json`. A variável opcional `CRYON_DATA_DIR` permite escolher uma pasta privada absoluta diferente, útil para testes isolados.

Bibliotecas e assets são compartilhados pelo cache; mundos, opções e configurações do jogo ficam no diretório de cada instância. Mudanças de versão na mesma instância podem alterar a compatibilidade dos mundos; use pastas distintas para aventuras em versões diferentes.

## Desenvolvimento

Pré-requisitos:

- Windows x64 com WebView2.
- Node.js LTS e npm no PATH (testado com Node 24.22.3).
- Rust estável via rustup e target MSVC x64 (testado com Rust/Cargo 1.100.0).
- Visual Studio Build Tools 2022, ferramentas desktop C++ e Windows SDK.

Na pasta do projeto:

```powershell
npm ci
npm run tauri dev
```

O modo de prévia no navegador mostra que o backend desktop está indisponível. Não implementa armazenamento alternativo nem downloads fictícios.

```powershell
# TypeScript e frontend de produção
npm run build

# Testes de regras, integridade, persistência e segurança
cargo test --manifest-path src-tauri/Cargo.toml --lib -j 1

# Integração HTTPS real: corrupção/recuperação e cancelamento
cargo test --manifest-path src-tauri/Cargo.toml --test network -j 1 -- --ignored --nocapture

# Formatação
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check

# Executável e instalador NSIS de produção
$env:CARGO_BUILD_JOBS = '1'
npm run tauri build -- --bundles nsis
```

Artefatos: `src-tauri/target/release/cryon.exe` e `src-tauri/target/release/bundle/nsis/`. O instalador é por usuário; os dados ficam fora da pasta do programa. As dependências estão fixadas em `package-lock.json` e `src-tauri/Cargo.lock`.

Em computadores com 4 GB de RAM, compile com um processo e encerre o jogo antes de compilar. A primeira compilação pode demorar; compilações posteriores reutilizam o cache.

## CLI de diagnóstico

O código também inclui `cryon-cli`, usando os mesmos serviços do aplicativo:

```powershell
cargo build --manifest-path src-tauri/Cargo.toml --bin cryon-cli -j 1
.\src-tauri\target\debug\cryon-cli.exe status
.\src-tauri\target\debug\cryon-cli.exe versions
.\src-tauri\target\debug\cryon-cli.exe install 1.16.5
.\src-tauri\target\debug\cryon-cli.exe launch 1.16.5
```

Os comandos `install` e `launch` selecionam uma instância da versão solicitada ou criam uma nova sem remover as demais. Feche o launcher gráfico antes de usar a CLI com a mesma pasta de dados. O comando de verificação `launch` acompanha o processo por até cinco minutos e solicita seu encerramento após esse tempo; para jogar normalmente, use o aplicativo gráfico.

## Organização do código

```text
src/App.tsx                        Navegação e conexão com os comandos
src/bridge.ts                      Tipos e ponte Tauri
src/components/InstanceDialog.tsx   Edição de instâncias
src/components/Preferences.tsx      Perfil, padrões, Java e diagnóstico
src/components/Transfer.tsx         Progresso verdadeiro e cancelamento
src/components/Crystal.tsx          Identidade vetorial original
src/components/Field.tsx            Campos reutilizáveis
src/styles.css, src/live.css        Layout, tema e estados
src-tauri/src/engine.rs             Estado, operação exclusiva, logs e progresso
src-tauri/src/settings.rs           JSON atômico, backup e recuperação
src-tauri/src/profiles.rs           Perfil offline e UUID
src-tauri/src/instances.rs          Instâncias, nomes e caminhos seguros
src-tauri/src/manifest.rs           Manifesto e metadados oficiais
src-tauri/src/downloads.rs          HTTPS, hashes, concorrência, extração e cancelamento
src-tauri/src/java.rs               Seleção, download e inspeção do runtime
src-tauri/src/minecraft.rs          Plano e instalação Vanilla
src-tauri/src/process.rs            Argumentos, processo Java e códigos de saída
src-tauri/src/errors.rs             Contexto de erros
src-tauri/src/lib.rs                Comandos Tauri e inicialização
src-tauri/tests/network.rs          Integração real de rede
```

## Limites da edição

O escopo é Vanilla com perfis offline no Windows x64. Não há Forge, Fabric, gerenciamento de mods/modpacks, OAuth Microsoft, atualização automática do próprio Cryon, assinatura digital do instalador ou suporte a outras plataformas. Versões antigas, snapshots e futuras mudanças dos metadados podem ter requisitos adicionais; falhas são informadas nos logs. As versões realmente verificadas e os resultados do instalador estão em `VALIDACAO.md`.

## Fontes e licenças

- [Manifesto oficial Mojang](https://piston-meta.mojang.com/mc/game/version_manifest_v2.json).
- [API Eclipse Adoptium](https://api.adoptium.net/).
- [Eclipse Temurin](https://adoptium.net/temurin/).
- [Preparação do ambiente Tauri](https://v2.tauri.app/start/prerequisites/).
- [Instaladores Windows Tauri](https://v2.tauri.app/distribute/windows-installer/).

Os arquivos do Minecraft e os runtimes Java não estão incluídos no pacote de fontes nem no instalador: são obtidos de suas fontes oficiais no primeiro uso. Os pacotes Temurin mantêm seus arquivos de licença e avisos; o runtime utiliza GPLv2 com Classpath Exception, conforme o pacote distribuído pelo fornecedor. Cryon é independente e não é afiliado à Mojang ou à Microsoft.

