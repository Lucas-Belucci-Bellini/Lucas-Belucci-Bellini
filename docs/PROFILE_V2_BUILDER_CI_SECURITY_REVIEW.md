# Revisão técnica dirigida — segurança do builder e gates de CI

**Data da inspeção:** 2026-10-28.  
**Refs comparadas:** `feat/profile-v2-consolidated`, `feat/v2p2-build-security`, `feat/v2p2-project-catalog`, `feat/v2p2-test-suite`, `feat/v2p2-deployment`.  
**Natureza:** inspeção estática dos arquivos remotos; não é resultado de execução dos testes.

## 1. Achado de alta prioridade: não substituir o builder de segurança pela versão do catálogo

O arquivo `scripts/build_portfolio.py` da branch `feat/v2p2-build-security` contém proteções adicionais que não aparecem na versão da branch `feat/v2p2-project-catalog` inspecionada:

- `_assert_safe_paths(root, out)` exige que a saída seja exatamente `dist/`, resolve a raiz e rejeita `dist/` como symlink.
- A leitura do catálogo rejeita symlink e exige arquivo regular.
- Cada asset estático é validado como arquivo regular e não symlink.
- A validação de URL limita portas HTTPS, rejeita fragmentos, normaliza ponto final do host, bloqueia sufixos locais/internos e valida endereços IP globais.
- O validador de GitHub exige host permitido e exatamente dois segmentos de caminho, sem query.

A versão de `feat/v2p2-project-catalog` não contém `_assert_safe_paths`, permite destinos de build mais amplos, não bloqueia symlinks nos inputs e tem validação de URL menos restritiva. Ela também aceita um conjunto mais estreito de hosts GitHub (`github.com` apenas), o que é uma decisão de produto que deve ser consistente com o contrato e os testes.

**Decisão:** usar #51 como ponto de partida do builder; reaplicar manualmente apenas a mudança de catálogo necessária do #54; nunca substituir o arquivo inteiro por uma cópia antiga/menos restritiva.

## 2. Achado de alta prioridade: a suíte completa e o frontend precisam continuar no workflow final

A branch `feat/v2p2-test-suite` altera o workflow para executar:

- `python -m unittest discover -s tests -v`
- `node --test tests/test_portfolio_frontend.cjs`

A branch `feat/v2p2-deployment` contém a suíte Python completa e verificações do artefato, mas o workflow consultado não apresenta a etapa Node `node --test tests/test_portfolio_frontend.cjs`.

**Decisão:** ao reconciliar #52 e #58, preservar as duas suítes. Verificar também que o job de build é requisito do job de deploy e que o upload de produção ocorre somente em push manualmente disparado para `main` ou `workflow_dispatch` na referência de produção, conforme as regras do workflow.

## 3. Contrato do artefato: a validação do workflow não substitui a validação do builder

O workflow de deploy verifica allowlist de arquivos, ausência de symlinks no artefato, formato dos dados e campos públicos permitidos. Essas verificações são valiosas, mas devem coexistir com os limites do próprio builder: a segurança não pode depender apenas de o workflow ser executado.

**Critério combinado:**
- O builder impõe destino exato `dist/` e protege arquivos de origem.
- O builder projeta apenas campos públicos e filtra sites não verificados.
- O workflow executa testes, constrói o artefato e verifica a allowlist final.
- Os testes incluem caminhos maliciosos, symlinks, URLs inválidas, privacidade, determinismo e testes frontend.
- O deploy permanece bloqueado até o job de build ser bem-sucedido.

## 4. Testes que precisam ser explicitamente mantidos ou acrescentados

- [ ] Destino de saída fora de `dist/` rejeitado, inclusive caminho absoluto externo.
- [ ] `dist/`, catálogo e assets não podem ser symlinks.
- [ ] URL HTTPS com porta inválida, porta não padrão, fragmento, credenciais ou destino privado rejeitada.
- [ ] URL GitHub com query, segmento extra, barra codificada ou domínio parecido rejeitada.
- [ ] Projeto privado, não visível ou sem flag explícita nunca é publicado.
- [ ] Website só é exportado com status `verified` e URL HTTPS segura.
- [ ] Rebuild determinístico remove artefatos antigos.
- [ ] Payload público contém somente a allowlist de campos.
- [ ] Suíte Python completa e testes Node frontend executados no CI.
- [ ] Workflow de deploy não publica em PR nem em branch de feature.

## 5. Próxima ação segura

Compor os arquivos de #51, #52, #54 e #58 em uma branch de integração isolada. Revisar o diff final consolidado, executar todas as suítes no ambiente de CI e verificar o artefato produzido. Não mesclar PRs em lote nem publicar o site durante esta fase.

## Limitações

Esta revisão identifica diferenças diretamente visíveis nos arquivos remotos; não afirma que algum teste falhou. Nenhum comando Python/Node foi executado nesta inspeção e nenhum deploy foi acionado.
