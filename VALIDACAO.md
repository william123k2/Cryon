# Validação do Cryon 0.2.1

**Resultado final: etapas 1–5 concluídas para o escopo Vanilla/offline no Windows x64.** Executável de produção compilado, instalador NSIS instalado com sucesso, perfil/instâncias persistentes e inicialização real confirmada em duas versões do Minecraft.

| Etapa | Resultado |
|---|---|
| 1 — base e interface | Build TypeScript/Vite e aplicativo Tauri real aprovados |
| 2 — persistência e instâncias | Testes de JSON, backup, UUID, reabertura e preservação de arquivos aprovados |
| 3 — catálogo, downloads e Java | Manifesto oficial, Java 8/21 privados, integridade, recuperação e cancelamento aprovados |
| 4 — instalação e jogo | Minecraft 1.16.5 e 1.21.4 alcançaram o menu principal e saíram com código 0 |
| 5 — distribuição Windows | Build release otimizado, instalador NSIS x64 e cópia instalada testados |

## Atualização 0.2.1

- Exibe um alerta persistente se o jogo encerrar inesperadamente, com o código de saída e as mensagens mais recentes do Java. A tentativa de abertura leva à tela de logs.
- Minimiza o launcher três segundos depois do processo do jogo continuar ativo; ao sair, restaura a janela ao estado anterior (normal, maximizada, tela cheia ou minimizada).
- Validação nesta atualização: TypeScript/Vite, nove testes Rust e build NSIS x64 passaram. O instalador 0.2.1 abriu a versão 1.16.5 até o menu principal; o launcher minimizou durante a execução e restaurou após a saída normal (código 0).

## Verificações já concluídas

- TypeScript estrito e build Vite de produção passaram.
- Nove testes Rust passaram: UUID offline compatível, validação de caminhos e nomes Windows reservados, regras de plataforma/recursos, argumentos com espaços e variáveis desconhecidas, detecção de hash incorreto, rejeição de ZIP com path traversal, persistência/backup, preservação de configurações corrompidas e trava exclusiva da pasta de dados.
- Teste de integração HTTPS com os servidores oficiais Mojang passou: download íntegro, corrupção de arquivo temporário próprio detectada, nova transferência corrigiu a corrupção, cancelamento interrompeu um download e não deixou JAR final nem arquivo `.part`.
- Teste de frontend com ponte Tauri simulada somente no teste passou para navegação, criação/edição/remoção de instâncias, confirmação/cancelamento, preferências, seleção de versão, instalar/cancelar e viewport de 640 px sem overflow ou erro JavaScript. Isso não substitui os testes desktop abaixo.
- Teste desktop real em `%LOCALAPPDATA%\Cryon-UI-Validation`: perfil alterado pela interface, salvo em JSON e confirmado após fechar/reabrir; UUID determinístico persistido.
- Instância criada pela interface e removida com confirmação; seleção recuperada e arquivo sentinela na pasta do jogo preservado.
- Seletor nativo de pasta abriu e cancelou sem alterar a configuração.
- Minecraft **1.16.5**, Java **8** gerenciado: instalação completa, 2.642 arquivos na primeira operação; cliente chegou ao menu principal, captura visual registrada, saída normal pelo botão Quit Game e código **0** confirmado.
- Java 8 e Java 21 baixados de Eclipse Adoptium Temurin, com SHA-256 conferido e arquivos legais preservados nos diretórios privados.
- Minecraft **1.21.4**, Java **21** gerenciado: instalação concluída, 4.130 arquivos planejados/verificados na operação; iniciou pelo botão **Jogar do Cryon de produção já instalado**, exibiu boas-vindas e menu principal, e encerrou pelo botão Quit Game com código **0**.
- Build de produção `tauri build --bundles nsis`: passou. A primeira compilação release retomada levou aproximadamente 21 minutos nesta máquina; o instalador gerado tem aproximadamente 3,72 MiB.
- Instalação silenciosa por usuário com o próprio pacote entregue: código **0**, programa em `%LOCALAPPDATA%\Programs\Cryon\cryon.exe` e atalho **Cryon** no menu Iniciar.
- Aplicativo instalado abriu e leu perfil, duas instâncias e os dois registros de versão instalada na pasta de dados final `%LOCALAPPDATA%\CryonData`.
- O programa não depende do Vite para executar. Frontend embutido no binário de produção, confirmado pela abertura da cópia instalada.

## Saída dos testes finais

```text
cargo test --lib --features tauri/custom-protocol -j 1
9 passed; 0 failed

cargo test --test network --features tauri/custom-protocol -j 1 -- --ignored --nocapture
1 passed; 0 failed

TypeScript + Vite build
1588 módulos; build concluído

NSIS: installerExit = 0; programExists = true
Minecraft 1.16.5: exit_code = 0
Minecraft 1.21.4: exit_code = 0
```

## Evidência do teste moderno

O recibo do runtime registra Temurin **21.0.13+9**, executável `jdk-21.0.13+9-jre/bin/java.exe`, e SHA-256 `2332c0b4cff6bed722da3567c91d5e47568bcba863fa4945d0a8d6fc76158628`. A captura `Cryon-Minecraft-1.21.4-menu.png` mostra o menu principal real.

O perfil principal permanece **Explorador**. Os testes de edição/remoção utilizaram outra pasta, `Cryon-UI-Validation`, sem misturar seus dados com a instalação entregue. Os dados de jogo preparados durante a implementação foram realocados de uma pasta provisória para `CryonData`, ajustando os caminhos dos cadastros e recibos. Ambos os runtimes foram conferidos com `java -version` após a realocação; o 1.21.4 foi iniciado por essa estrutura final.

## Evidência do teste legado

O Java 8 não foi obtido do Java instalado no sistema. O recibo privado registra Temurin `jdk8u512-b09`, executável `jdk8u512-b09-jre/bin/java.exe`, URL pública oficial e SHA-256 `ae1ae12f817d7735477c6555107eece73854da0654da69077309ef9bff0b78bf`.

A captura `Cryon-Minecraft-1.16.5-menu.png` na pasta de outputs mostra o cliente real no menu. Os logs na pasta privada registram inicialização, bibliotecas gráficas, assets e código de saída. Na entrega final, os dados ficam em `%LOCALAPPDATA%\CryonData`, separados dos arquivos do programa.

## Limitações observadas no ambiente

O computador tem cerca de 4 GB de RAM. Uma chamada de automação expirou ao abrir Minecraft enquanto o compilador de produção e a interface de teste estavam ativos. A compilação foi interrompida de forma segura para liberar recursos; com o jogo no primeiro plano, a captura e o encerramento normal funcionaram. A compilação foi retomada depois. Não foi interpretado como falha funcional do launcher.

Microsoft OAuth e autenticação de servidores oficiais não fazem parte do escopo offline. Não foram implementados nem declarados como funcionais. Nenhum teste de conexão a servidor autenticado foi executado.

Os testes gráficos confirmam chegada ao menu e encerramento; não incluem sessão prolongada, criação de mundo, multiplayer, todas as versões do catálogo, outros computadores ou desinstalação. O instalador é sem assinatura digital. Esses limites não são apresentados como funcionalidades verificadas.
